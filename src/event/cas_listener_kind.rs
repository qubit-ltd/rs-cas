// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Listener locations used by CAS diagnostics.

/// Location at which an observation listener failed.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasEvent;
/// use qubit_cas::CasHooks;
/// use qubit_cas::CasListenerKind;
///
/// // Listener isolation applies to unwind builds.
/// let hooks = CasHooks::new().on_event(|event: &CasEvent| {
///     if matches!(event, CasEvent::ExecutionStarted { .. }) {
///         panic!("metrics unavailable");
///     }
/// });
/// let state = AtomicRef::from_value(3usize);
/// let outcome = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute_with_hooks(&state, |_: &usize| CasDecision::finish(()), hooks);
/// assert!(outcome.is_ok());
/// let failure = &outcome.report().listener_failures()[0];
/// assert_eq!(failure.kind(), CasListenerKind::ExecutionStarted);
/// assert_eq!(failure.message(), "metrics unavailable");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasListenerKind {
    /// Execution-started event listener.
    ExecutionStarted,
    /// Attempt-failed event listener.
    AttemptFailed,
    /// Retry-scheduled event listener.
    RetryScheduled,
    /// Execution-finished event listener.
    ExecutionFinished,
    /// Contention alert listener.
    ContentionAlert,
}
