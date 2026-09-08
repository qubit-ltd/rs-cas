// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tokio-gated asynchronous CAS execution entry points and attempts.

use std::sync::Arc;
use std::sync::Mutex;

use qubit_atomic::AtomicRef;

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
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    #[cfg(feature = "tokio")]
    pub async fn execute_async<R, O, Fut>(&self, state: &AtomicRef<T>, operation: O) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
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
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    ///
    /// # Returns
    /// The terminal CAS success or error without an execution report.
    ///
    /// # Cancellation
    /// Cancelling the returned future cancels the in-flight operation future.
    /// Operations must therefore remain safe to retry or cancel.
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
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let attempt_snapshot = Arc::new(Mutex::new(None));
        let attempt_snapshot_for_attempt = Arc::clone(&attempt_snapshot);
        let mut async_retry = self.result_retry().tokio();
        if let Some(timeout) = self.attempt_timeout {
            async_retry = async_retry.hard_attempt_timeout(timeout);
        }
        if let Some(timeout) = self.flow_timeout() {
            async_retry = async_retry.hard_flow_timeout(timeout);
        }
        let attempt = async_retry
            .run(|| run_async_attempt(state, &operation, Arc::clone(&attempt_snapshot_for_attempt)))
            .await;
        match attempt {
            Ok(success) => {
                // This adapter registers no completion observers; only retry context is
                // projected.
                let (success, context, _diagnostics) = success.into_parts();
                Ok(super::finalization::enrich_success(success, context))
            }
            Err(error) => {
                let timeout_current = attempt_snapshot
                    .lock()
                    .expect("CAS attempt snapshot slot should be lockable")
                    .clone();
                Err(CasError::new(error, timeout_current))
            }
        }
    }

    /// Executes one asynchronous CAS operation with lifecycle hooks.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Async operation factory receiving one state snapshot.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
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
        Fut: std::future::Future<Output = CasDecision<T, R, E>>,
    {
        let attempt_snapshot = Arc::new(Mutex::new(None));
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(&hooks, Arc::clone(&report_builder));
        let attempt_snapshot_for_attempt = Arc::clone(&attempt_snapshot);
        let mut async_retry = retry.tokio();
        if let Some(timeout) = self.attempt_timeout {
            async_retry = async_retry.hard_attempt_timeout(timeout);
        }
        if let Some(timeout) = self.flow_timeout() {
            async_retry = async_retry.hard_flow_timeout(timeout);
        }
        let attempt = async_retry
            .run(|| run_async_attempt(state, &operation, Arc::clone(&attempt_snapshot_for_attempt)))
            .await;
        self.finish_execution(attempt, hooks, Some(attempt_snapshot), report_builder)
    }
}

/// Runs one asynchronous attempt and records its snapshot for timeout errors.
#[cfg(feature = "tokio")]
async fn run_async_attempt<T, R, E, O, Fut>(
    state: &AtomicRef<T>,
    operation: &O,
    attempt_snapshot: Arc<Mutex<Option<Arc<T>>>>,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
where
    O: Fn(Arc<T>) -> Fut,
    Fut: std::future::Future<Output = CasDecision<T, R, E>>,
{
    let current = state.load();
    *attempt_snapshot
        .lock()
        .expect("CAS attempt snapshot slot should be lockable") = Some(Arc::clone(&current));
    let decision = operation(Arc::clone(&current)).await;
    apply_decision(state, current, decision)
}
