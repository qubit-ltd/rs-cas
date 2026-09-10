// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS terminal classifications.

use super::CasLimitKind;
use super::CasTimeoutScope;

/// Structured reason why a CAS execution stopped.
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
/// assert_eq!(error.termination(), CasTermination::LimitExceeded(CasLimitKind::Attempts));
/// assert_eq!(error.error(), Some(&"busy"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasTermination {
    /// The operation explicitly aborted.
    Aborted,
    /// A retry limit prevented another attempt.
    LimitExceeded(
        /// Admission limit that prevented continued execution.
        CasLimitKind,
    ),
    /// A hard timeout stopped execution.
    TimedOut(
        /// Scope of the cooperative deadline that stopped the future.
        CasTimeoutScope,
    ),
    /// The retry infrastructure could not continue safely.
    RetryInfrastructure,
}
