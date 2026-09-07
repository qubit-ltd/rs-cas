// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous CAS execution entry points and attempt support.

use std::sync::Arc;
use std::sync::Mutex;

use qubit_atomic::AtomicRef;
use qubit_function::Function;

use super::CasExecutor;
use crate::CasError;
use crate::CasHooks;
use crate::CasOutcome;
use crate::CasSuccess;
use crate::cas_decision::CasDecision;
use crate::error::CasAttemptFailure;
use crate::executor::cas_executor::decision::apply_decision;
use crate::executor::internal::AttemptSuccess;
use crate::report::CasReportBuilder;

impl<T, E> CasExecutor<T, E> {
    /// Executes one synchronous CAS operation.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and
    ///   returns a CAS decision.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Blocking
    /// Configured retry delays block the calling thread until execution ends.
    pub fn execute<R, O>(&self, state: &AtomicRef<T>, operation: O) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        self.execute_with_hooks(state, operation, CasHooks::new())
    }

    /// Executes one synchronous CAS operation without constructing a report.
    ///
    /// This path preserves retry, success, and terminal error semantics while
    /// skipping report accumulation and lifecycle hook dispatch. Use
    /// [`Self::execute`] when the caller needs execution metrics.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and
    ///   returns a CAS decision.
    ///
    /// # Returns
    /// The terminal CAS success or error without an execution report.
    ///
    /// # Blocking
    /// Configured retry delays block the calling thread until execution ends.
    pub fn execute_result<R, O>(&self, state: &AtomicRef<T>, operation: O) -> Result<CasSuccess<T, R>, CasError<T, E>>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        let attempt = self.result_retry().sync().run(|| run_sync_attempt(state, &operation));
        match attempt {
            Ok(success) => {
                // This adapter registers no completion observers; only retry context is
                // projected.
                let (success, context, _diagnostics) = success.into_parts();
                Ok(super::finalization::enrich_success(success, context))
            }
            Err(error) => Err(CasError::new(error, None)),
        }
    }

    /// Executes one synchronous CAS operation with lifecycle hooks.
    ///
    /// # Parameters
    /// - `state`: Shared atomic state container.
    /// - `operation`: Pure operation that inspects the current state and
    ///   returns a CAS decision.
    /// - `hooks`: Per-execution hook registrations.
    ///
    /// # Returns
    /// A terminal result together with the execution report.
    ///
    /// # Blocking
    /// Configured retry delays block the calling thread until execution ends.
    ///
    /// # Panics
    /// With [`crate::observability::ListenerPanicPolicy::Propagate`], panics
    /// from outer `ExecutionStarted`/`ExecutionFinished` listeners and
    /// alert listeners unwind through this call. Panics from retry-owned
    /// `AttemptFailed` and `RetryRequested` listeners instead return a
    /// [`crate::CasRetryFailure::CallbackFailed`] terminal error.
    /// [`crate::observability::ListenerPanicPolicy::Isolate`] catches every
    /// listener panic at dispatch and allows execution to continue.
    pub fn execute_with_hooks<R, O>(&self, state: &AtomicRef<T>, operation: O, hooks: CasHooks) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
        O: Function<T, CasDecision<T, R, E>>,
    {
        let report_builder = Arc::new(Mutex::new(CasReportBuilder::start()));
        self.emit_started(&hooks, &report_builder);
        let retry = self.build_retry(&hooks, Arc::clone(&report_builder));
        let attempt = retry.sync().run(|| run_sync_attempt(state, &operation));
        self.finish_execution(attempt, hooks, None, report_builder)
    }
}

/// Runs one synchronous attempt after loading its state snapshot.
pub(super) fn run_sync_attempt<T, R, E, O>(
    state: &AtomicRef<T>,
    operation: &O,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
where
    O: Function<T, CasDecision<T, R, E>>,
{
    let current = state.load();
    let decision = operation.apply(current.as_ref());
    apply_decision(state, current, decision)
}
