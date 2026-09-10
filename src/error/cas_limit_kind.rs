// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS retry limit categories.

/// Limit that prevented another CAS attempt.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasLimitKind;
/// use qubit_cas::CasTermination;
///
/// let state = AtomicRef::from_value(3usize);
/// let error = CasExecutor::<usize, &'static str>::builder().max_attempts(1).build().unwrap()
///     .execute_result(&state, |_: &usize| CasDecision::<usize, (), _>::retry("busy"))
///     .unwrap_err();
/// assert!(matches!(error.termination(), CasTermination::LimitExceeded(CasLimitKind::Attempts)));
/// assert_eq!(error.attempts(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasLimitKind {
    /// Maximum number of attempts.
    Attempts,
    /// Cumulative attempt time, including operation and CAS adapter work.
    OperationElapsed,
    /// Total flow elapsed time.
    TotalElapsed,
}
