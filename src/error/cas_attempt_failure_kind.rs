// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lightweight kind of attempt-level CAS failure.

/// Lightweight kind of attempt-level CAS failure.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasAttemptFailureKind;
///
/// let state = AtomicRef::from_value(3usize);
/// let error = CasExecutor::<usize, &'static str>::builder().build().unwrap()
///     .execute_result(&state, |_: &usize| CasDecision::<usize, (), _>::abort("sold out"))
///     .unwrap_err();
/// assert_eq!(error.last_failure().unwrap().kind(), CasAttemptFailureKind::Abort);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasAttemptFailureKind {
    /// Compare-and-swap failed because another writer changed the state first.
    Conflict,
    /// Business logic requested another attempt.
    Retry,
    /// Business logic aborted the flow.
    Abort,
    /// An async attempt exceeded its timeout.
    Timeout,
}
