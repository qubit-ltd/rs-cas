// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Terminal outcomes represented in CAS execution reports.

/// Terminal outcome captured in a CAS execution report.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasExecutionOutcome;
///
/// let state = AtomicRef::from_value(3usize);
/// let outcome = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute(&state, |_: &usize| CasDecision::finish("available"));
/// assert_eq!(outcome.report().outcome(), CasExecutionOutcome::SuccessFinished);
/// assert_eq!(outcome.report().attempts_total(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasExecutionOutcome {
    /// The flow completed by installing a new state.
    SuccessUpdated,
    /// The flow completed successfully without writing.
    SuccessFinished,
    /// The flow aborted because business logic requested an abort.
    ErrorAbort,
    /// Retry limits were exhausted by compare-and-swap conflicts.
    ErrorConflictExhausted,
    /// Retry limits were exhausted by retryable business failures.
    ErrorRetryExhausted,
    /// The flow stopped because an async attempt timed out.
    ErrorAttemptTimeout,
    /// The hard whole-flow timeout fired.
    ErrorFlowTimeout,
    /// Retry infrastructure failed while scheduling or stopping work.
    ErrorRetryInfrastructure,
    /// The cumulative attempt elapsed-time budget was exceeded.
    ErrorOperationBudgetExceeded,
    /// The monotonic total retry-flow elapsed-time budget was exceeded.
    ErrorTotalBudgetExceeded,
}
