// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Classified kinds for terminal CAS errors.

/// Classified reason for a terminal CAS error.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasErrorKind;
///
/// let state = AtomicRef::from_value(3usize);
/// let error = CasExecutor::<usize, &'static str>::builder().build().unwrap()
///     .execute_result(&state, |_: &usize| CasDecision::<usize, (), _>::abort("sold out"))
///     .unwrap_err();
/// assert_eq!(error.kind(), CasErrorKind::Abort);
/// assert_eq!(error.error(), Some(&"sold out"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasErrorKind {
    /// The operation explicitly aborted.
    Abort,
    /// Retry limits were exhausted by compare-and-swap conflicts.
    ConflictExhausted,
    /// Retry limits were exhausted by retryable business failures.
    RetryExhausted,
    /// A single attempt timed out.
    AttemptTimeout,
    /// The hard whole-flow timeout fired.
    FlowTimeout,
    /// The retry layer failed while scheduling or stopping retry work.
    RetryInfrastructure,
    /// The cumulative attempt elapsed-time budget expired.
    OperationBudgetExceeded,
    /// The monotonic total retry-flow elapsed-time budget expired.
    TotalBudgetExceeded,
}
