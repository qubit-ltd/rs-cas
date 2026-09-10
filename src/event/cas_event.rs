// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS lifecycle event payload.

use std::time::Duration;
use std::time::Instant;

use super::CasContext;
use crate::error::CasAttemptFailureKind;
use crate::report::CasExecutionReport;

/// Lifecycle event emitted by a CAS execution.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicUsize;
/// use std::sync::atomic::Ordering;
///
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasEvent;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasHooks;
///
/// let finished = Arc::new(AtomicUsize::new(0));
/// let observed = Arc::clone(&finished);
/// let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
///     if let CasEvent::ExecutionFinished { report } = event {
///         observed.store(report.attempts_total() as usize, Ordering::SeqCst);
///     }
/// });
/// let state = AtomicRef::from_value(3usize);
/// let outcome = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute_with_hooks(&state, |_: &usize| CasDecision::finish(()), hooks);
/// assert!(outcome.is_ok());
/// assert_eq!(finished.load(Ordering::SeqCst), 1);
/// ```
#[derive(Debug, Clone)]
pub enum CasEvent {
    /// The execution started before the first attempt.
    ExecutionStarted {
        /// Instant captured when the execution started.
        started_at: Instant,
    },

    /// One attempt failed.
    AttemptFailed {
        /// Context captured for the failed attempt.
        context: CasContext,
        /// Attempt-level failure kind.
        kind: CasAttemptFailureKind,
    },

    /// A retry passed the scheduling checks and has a selected backoff delay.
    ///
    /// This event is not emitted when the attempt limit or a scheduling-time
    /// budget check already prevents a retry. A later deadline, cancellation,
    /// or admission check can still prevent the next operation from starting.
    /// Use the terminal attempt count to measure executed operations.
    RetryScheduled {
        /// Context captured after the failed attempt.
        context: CasContext,
        /// Delay selected before the next attempt.
        delay: Duration,
    },

    /// The execution finished and produced a report.
    ExecutionFinished {
        /// Terminal snapshot captured before completion and alert callbacks;
        /// panics from those callbacks appear only in the returned report.
        report: CasExecutionReport,
    },
}
