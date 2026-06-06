// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Classified kinds for terminal CAS errors.

/// Classified reason for a terminal CAS error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasErrorKind {
    /// The operation explicitly aborted.
    Abort,
    /// Retry limits were exhausted by compare-and-swap conflicts.
    Conflict,
    /// Retry limits were exhausted by retryable business failures.
    RetryExhausted,
    /// A timeout aborted the flow or exhausted retry limits.
    AttemptTimeout,
    /// The cumulative user operation elapsed-time budget expired.
    MaxOperationElapsedExceeded,
    /// The monotonic total retry-flow elapsed-time budget expired.
    MaxTotalElapsedExceeded,
}
