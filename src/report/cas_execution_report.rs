/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::time::{
    Duration,
    Instant,
};

use crate::observability::ContentionThresholds;

use super::CasExecutionOutcome;

/// Immutable report describing one completed CAS execution.
#[derive(Debug, Clone)]
pub struct CasExecutionReport {
    /// Total attempts executed by the retry loop.
    attempts_total: u32,
    /// Number of compare-and-swap conflicts.
    conflicts: u32,
    /// Number of retryable business failures.
    retry_errors: u32,
    /// Number of aborting business failures.
    aborts: u32,
    /// Number of async attempt timeouts.
    timeouts: u32,
    /// Instant captured before the first attempt.
    started_at: Instant,
    /// Instant captured when the flow completed.
    finished_at: Instant,
    /// Configured maximum attempts.
    max_attempts: u32,
    /// Configured maximum cumulative user operation time.
    max_operation_elapsed: Option<Duration>,
    /// Configured maximum total retry-flow elapsed time.
    max_total_elapsed: Option<Duration>,
    /// Terminal outcome.
    outcome: CasExecutionOutcome,
}

impl CasExecutionReport {
    /// Creates one report from its raw parts.
    ///
    /// This constructor is used internally by the report builder to produce the
    /// final immutable summary after a CAS execution completes.
    ///
    /// # Parameters
    /// - `attempts_total`: Total number of attempts executed by the retry loop
    ///   (one-based).
    /// - `conflicts`: Number of compare-and-swap conflicts encountered.
    /// - `retry_errors`: Number of retryable business failures.
    /// - `aborts`: Number of business aborts.
    /// - `timeouts`: Number of async attempt timeouts (when timeout policy is
    ///   configured).
    /// - `started_at`: Instant captured before the first attempt.
    /// - `finished_at`: Instant captured when the flow completed.
    /// - `max_attempts`: Configured maximum number of attempts.
    /// - `max_operation_elapsed`: Configured maximum cumulative user operation
    ///   time, if any.
    /// - `max_total_elapsed`: Configured maximum total retry-flow elapsed time,
    ///   if any.
    /// - `outcome`: The terminal [`CasExecutionOutcome`] of the execution.
    ///
    /// # Returns
    /// A fully populated [`CasExecutionReport`] value.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        attempts_total: u32,
        conflicts: u32,
        retry_errors: u32,
        aborts: u32,
        timeouts: u32,
        started_at: Instant,
        finished_at: Instant,
        max_attempts: u32,
        max_operation_elapsed: Option<Duration>,
        max_total_elapsed: Option<Duration>,
        outcome: CasExecutionOutcome,
    ) -> Self {
        Self {
            attempts_total,
            conflicts,
            retry_errors,
            aborts,
            timeouts,
            started_at,
            finished_at,
            max_attempts,
            max_operation_elapsed,
            max_total_elapsed,
            outcome,
        }
    }

    /// Returns total attempts executed by the retry loop.
    ///
    /// # Returns
    /// One-based count of attempts performed (including the successful or
    /// terminal one).
    #[inline]
    pub fn attempts_total(&self) -> u32 {
        self.attempts_total
    }

    /// Returns the number of compare-and-swap conflicts.
    ///
    /// # Returns
    /// Count of times a CAS operation failed due to state change by another
    /// thread/process.
    #[inline]
    pub fn conflicts(&self) -> u32 {
        self.conflicts
    }

    /// Returns the number of retryable business failures.
    ///
    /// # Returns
    /// Count of times the business operation returned
    /// [`CasDecision::Retry`](crate::CasDecision::Retry).
    #[inline]
    pub fn retry_errors(&self) -> u32 {
        self.retry_errors
    }

    /// Returns the number of aborting business failures.
    ///
    /// # Returns
    /// Count of times the business operation returned
    /// [`CasDecision::Abort`](crate::CasDecision::Abort).
    #[inline]
    pub fn aborts(&self) -> u32 {
        self.aborts
    }

    /// Returns the number of async attempt timeouts.
    ///
    /// # Returns
    /// Count of times an async operation exceeded its configured timeout.
    #[inline]
    pub fn timeouts(&self) -> u32 {
        self.timeouts
    }

    /// Returns the instant captured before the first attempt.
    ///
    /// # Returns
    /// [`Instant`] at the start of the CAS execution.
    #[inline]
    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Returns the instant captured when execution completed.
    ///
    /// # Returns
    /// [`Instant`] when the CAS flow terminated.
    #[inline]
    pub fn finished_at(&self) -> Instant {
        self.finished_at
    }

    /// Returns elapsed wall-clock time for the execution.
    ///
    /// # Returns
    /// Duration between `started_at` and `finished_at`.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.finished_at.duration_since(self.started_at)
    }

    /// Returns the configured maximum attempts.
    ///
    /// # Returns
    /// The `max_attempts` value used by the retry policy.
    #[inline]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the configured maximum cumulative user operation time budget.
    ///
    /// # Returns
    /// `Some(Duration)` if a budget was set, otherwise `None`.
    #[inline]
    pub fn max_operation_elapsed(&self) -> Option<Duration> {
        self.max_operation_elapsed
    }

    /// Returns the configured maximum total retry-flow elapsed-time budget.
    ///
    /// # Returns
    /// `Some(Duration)` if a budget was set, otherwise `None`.
    #[inline]
    pub fn max_total_elapsed(&self) -> Option<Duration> {
        self.max_total_elapsed
    }

    /// Returns the terminal outcome captured in this report.
    ///
    /// # Returns
    /// The [`CasExecutionOutcome`] classifying how the execution ended.
    #[inline]
    pub fn outcome(&self) -> CasExecutionOutcome {
        self.outcome
    }

    /// Returns conflicts divided by total attempts.
    ///
    /// # Returns
    /// The conflict ratio in range `[0.0, 1.0]`. Returns `0.0` if no attempts
    /// were made.
    #[inline]
    pub fn conflict_ratio(&self) -> f64 {
        if self.attempts_total == 0 {
            0.0
        } else {
            self.conflicts as f64 / self.attempts_total as f64
        }
    }

    /// Returns retryable business failures divided by total attempts.
    ///
    /// # Returns
    /// The retryable failure ratio in range `[0.0, 1.0]`. Returns `0.0` if no
    /// attempts were made.
    #[inline]
    pub fn retryable_failure_ratio(&self) -> f64 {
        if self.attempts_total == 0 {
            0.0
        } else {
            self.retry_errors as f64 / self.attempts_total as f64
        }
    }

    /// Returns whether the report crosses the configured contention threshold.
    ///
    /// # Parameters
    /// - `thresholds`: The [`ContentionThresholds`] to check against.
    ///
    /// # Returns
    /// `true` if `attempts_total`, `conflicts` and `conflict_ratio` all meet or
    /// exceed the thresholds (suitable for triggering alerts).
    #[inline]
    pub fn is_contention_hot(&self, thresholds: &ContentionThresholds) -> bool {
        self.attempts_total >= thresholds.min_attempts()
            && self.conflicts >= thresholds.min_conflicts()
            && self.conflict_ratio() >= thresholds.conflict_ratio()
    }
}
