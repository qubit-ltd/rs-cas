// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS timeout scopes.

/// Scope of a timeout that terminated CAS execution.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasTermination;
/// use qubit_cas::CasTimeoutScope;
///
/// let state = AtomicRef::from_value(3usize);
/// let error = CasExecutor::<usize, &'static str>::builder().max_attempts(1).build().unwrap()
///     .execute_result(&state, |_: &usize| CasDecision::<usize, (), _>::retry("busy"))
///     .unwrap_err();
/// let scope = match error.termination() {
///     CasTermination::TimedOut(CasTimeoutScope::Attempt) => Some("attempt"),
///     CasTermination::TimedOut(CasTimeoutScope::Flow) => Some("flow"),
///     _ => None,
/// };
/// assert_eq!(scope, None, "attempt exhaustion is not a timeout");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasTimeoutScope {
    /// One operation attempt.
    Attempt,
    /// The whole retry flow.
    Flow,
}
