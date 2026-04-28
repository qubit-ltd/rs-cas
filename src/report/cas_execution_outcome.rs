/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

/// Terminal outcome captured in a CAS execution report.
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
    /// The cumulative user operation elapsed-time budget was exceeded.
    ErrorMaxOperationElapsedExceeded,
    /// The monotonic total retry-flow elapsed-time budget was exceeded.
    ErrorMaxTotalElapsedExceeded,
}
