// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tokio-gated asynchronous CAS execution entry points and attempts.

use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;

use qubit_atomic::AtomicRef;
use qubit_retry::TokioRetry;

use super::CasExecutor;
use crate::cas_decision::CasDecision;
use crate::cas_outcome::CasOutcome;
use crate::cas_success::CasSuccess;
use crate::error::CasAttemptFailure;
use crate::error::CasError;
use crate::event::CasHooks;
use crate::executor::cas_executor::decision::apply_decision;
use crate::executor::internal::AttemptSuccess;
use crate::report::CasReportBuilder;

impl<T, E> CasExecutor<T, E> {
    /// Executes one asynchronous CAS operation.
    ///
    /// # Type Parameters
    /// - `R`: Output moved to the caller only after a successful attempt.
    /// - `O`: Factory receiving an owned snapshot for each async attempt.
    /// - `Fut`: Future yielding that attempt's CAS decision.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Cancellation
    /// Dropping the future drops the in-flight operation without rolling back
    /// committed state or external effects. Timeouts require yielding futures.
    ///
    /// # Panics
    /// An operation panic propagates to the caller.
    #[cfg(feature = "tokio")]
    #[inline(always)]
    pub async fn execute_async<R, O, Fut>(&self, state: &AtomicRef<T>, operation: O) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: Future<Output = CasDecision<T, R, E>>,
    {
        self.execute_async_with_hooks(state, operation, CasHooks::new()).await
    }

    /// Executes one asynchronous CAS operation without constructing a report.
    ///
    /// This path preserves retry, success, timeout, and terminal error
    /// semantics while skipping report accumulation and lifecycle hook
    /// dispatch. Use [`Self::execute_async`] when the caller needs execution
    /// metrics.
    ///
    /// # Type Parameters
    /// - `R`: Output moved to the caller only after a successful attempt.
    /// - `O`: Factory receiving an owned snapshot for each async attempt.
    /// - `Fut`: Future yielding that attempt's CAS decision.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// The terminal CAS success or error without an execution report.
    ///
    /// # Errors
    /// Returns a business abort, exhausted retry or soft-budget limit, timeout,
    /// or infrastructure failure with the last available attempt snapshot.
    ///
    /// # Cancellation
    /// Cancelling the returned future cancels the in-flight operation future.
    /// Operations must therefore remain safe to retry or cancel. Cancellation
    /// does not roll back committed state or external effects. Hard deadlines
    /// cannot preempt blocking code or futures that never yield.
    ///
    /// # Panics
    /// An operation panic propagates to the caller.
    #[cfg(feature = "tokio")]
    pub async fn execute_async_result<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
    ) -> Result<CasSuccess<T, R>, CasError<T, E>>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: Future<Output = CasDecision<T, R, E>>,
    {
        let attempt_snapshot = Mutex::new(None);
        let snapshot_slot =
            (self.attempt_timeout.is_some() || self.flow_timeout().is_some()).then_some(&attempt_snapshot);
        let mut async_retry = TokioRetry::new(self.result_retry());
        if let Some(timeout) = self.attempt_timeout {
            async_retry = async_retry.hard_attempt_timeout(timeout);
        }
        if let Some(timeout) = self.flow_timeout() {
            async_retry = async_retry.hard_flow_timeout(timeout);
        }
        let attempt = async_retry
            .run(|| run_async_attempt(state, &operation, snapshot_slot))
            .await;
        match attempt {
            Ok(success) => {
                // This adapter registers no completion observers; only retry context is
                // projected.
                let (success, context, diagnostics) = success.into_parts();
                debug_assert!(diagnostics.is_empty(), "CAS installs no completion callbacks");
                Ok(super::finalization::enrich_success(success, context))
            }
            Err(error) => {
                let timeout_current = attempt_snapshot
                    .into_inner()
                    .expect("CAS attempt snapshot slot should be lockable");
                Err(CasError::new(error, timeout_current))
            }
        }
    }

    /// Executes one asynchronous CAS operation with lifecycle hooks.
    ///
    /// # Type Parameters
    /// - `R`: Output moved to the caller only after a successful attempt.
    /// - `O`: Factory receiving an owned snapshot for each async attempt.
    /// - `Fut`: Future yielding that attempt's CAS decision.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Cancellation
    /// Dropping the future drops the in-flight operation without rolling back
    /// committed state or external effects. Timeouts require yielding futures.
    ///
    /// # Panics
    /// An operation panic propagates to the caller.
    ///
    /// Listener panics are isolated and recorded in the execution report;
    /// they do not alter the CAS terminal result.
    #[cfg(feature = "tokio")]
    pub async fn execute_async_with_hooks<R, O, Fut>(
        &self,
        state: &AtomicRef<T>,
        operation: O,
        hooks: CasHooks,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: Future<Output = CasDecision<T, R, E>>,
    {
        let attempt_snapshot = Mutex::new(None);
        let snapshot_slot =
            (self.attempt_timeout.is_some() || self.flow_timeout().is_some()).then_some(&attempt_snapshot);
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(&hooks, Arc::clone(&report_builder));
        let mut async_retry = TokioRetry::new(&retry);
        if let Some(timeout) = self.attempt_timeout {
            async_retry = async_retry.hard_attempt_timeout(timeout);
        }
        if let Some(timeout) = self.flow_timeout() {
            async_retry = async_retry.hard_flow_timeout(timeout);
        }
        let attempt = async_retry
            .run(|| run_async_attempt(state, &operation, snapshot_slot))
            .await;
        let timeout_current = attempt_snapshot
            .into_inner()
            .expect("CAS attempt snapshot slot should be lockable");
        self.finish_execution(attempt, hooks, timeout_current, report_builder)
    }
}

/// Runs one asynchronous attempt and records its snapshot for timeout errors.
///
/// # Type Parameters
/// - `T`: Shared state held alive through the attempt.
/// - `R`: Output returned by a successful operation.
/// - `E`: Business failure returned by the operation.
/// - `O`: Factory creating a fresh future for each attempt.
/// - `Fut`: Future yielding a CAS decision.
///
/// # Parameters
/// - `state`: Slot loaded when this attempt is first polled.
/// - `operation`: Factory receiving shared ownership of the loaded snapshot.
/// - `attempt_snapshot`: `Some` retains a snapshot for timeout projection;
///   `None` avoids this bookkeeping when no timeout is configured.
///
/// # Returns
/// A successful update or no-write finish after the operation resolves.
///
/// # Errors
/// Returns a CAS conflict or an explicit Retry/Abort failure.
///
/// # Panics
/// Operation panics propagate; a poisoned snapshot mutex also panics.
/// The mutex is released before polling the operation.
///
/// # Cancellation
/// Dropping this future drops the operation without publishing its decision;
/// external side effects already performed by the operation are not undone.
#[cfg(feature = "tokio")]
async fn run_async_attempt<T, R, E, O, Fut>(
    state: &AtomicRef<T>,
    operation: &O,
    attempt_snapshot: Option<&Mutex<Option<Arc<T>>>>,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
where
    O: Fn(Arc<T>) -> Fut,
    Fut: Future<Output = CasDecision<T, R, E>>,
{
    let current = state.load();
    if let Some(slot) = attempt_snapshot {
        *slot.lock().expect("CAS attempt snapshot slot should be lockable") = Some(Arc::clone(&current));
    }
    let decision = operation(Arc::clone(&current)).await;
    apply_decision(state, current, decision)
}
