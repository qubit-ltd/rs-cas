// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::{
    Duration,
    Instant,
};

use super::{
    CasExecutionOutcome,
    CasExecutionReport,
};

/// Mutable accumulator used internally while one CAS flow is running.
#[derive(Debug, Clone)]
pub(crate) struct CasReportBuilder {
    started_at: Instant,
    conflicts: u32,
    retry_errors: u32,
    aborts: u32,
    timeouts: u32,
}

impl CasReportBuilder {
    /// Starts one report accumulator with the current instant and zeroed
    /// counters.
    ///
    /// # Returns
    /// A fresh [`CasReportBuilder`] ready to record statistics during a CAS
    /// execution.
    #[inline]
    pub(crate) fn start() -> Self {
        Self {
            started_at: Instant::now(),
            conflicts: 0,
            retry_errors: 0,
            aborts: 0,
            timeouts: 0,
        }
    }

    /// Returns the start instant captured for this report.
    ///
    /// # Returns
    /// The [`Instant`] when this report builder was started.
    #[inline]
    pub(crate) fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Records one compare-and-swap conflict.
    ///
    /// Increments the internal conflict counter using saturating arithmetic to
    /// prevent overflow.
    #[inline]
    pub(crate) fn record_conflict(&mut self) {
        self.conflicts = self.conflicts.saturating_add(1);
    }

    /// Records one retryable business failure.
    ///
    /// Increments the internal retry error counter using saturating arithmetic.
    #[inline]
    pub(crate) fn record_retry_error(&mut self) {
        self.retry_errors = self.retry_errors.saturating_add(1);
    }

    /// Records one aborting business failure.
    ///
    /// Increments the internal abort counter using saturating arithmetic.
    #[inline]
    pub(crate) fn record_abort(&mut self) {
        self.aborts = self.aborts.saturating_add(1);
    }

    /// Records one async attempt timeout.
    ///
    /// Increments the internal timeout counter using saturating arithmetic.
    #[inline]
    pub(crate) fn record_timeout(&mut self) {
        self.timeouts = self.timeouts.saturating_add(1);
    }

    /// Finishes the accumulator into an immutable report.
    ///
    /// # Parameters
    /// - `attempts_total`: The total number of attempts performed (provided by
    ///   the retry layer).
    /// - `max_attempts`: Configured maximum attempts.
    /// - `max_operation_elapsed`: Configured cumulative user operation time
    ///   budget.
    /// - `max_total_elapsed`: Configured total retry-flow time budget.
    /// - `outcome`: The terminal outcome determined for this execution.
    ///
    /// # Returns
    /// A completed [`CasExecutionReport`] with all statistics and timing
    /// information.
    #[inline]
    pub(crate) fn finish(
        &self,
        attempts_total: u32,
        max_attempts: u32,
        max_operation_elapsed: Option<Duration>,
        max_total_elapsed: Option<Duration>,
        outcome: CasExecutionOutcome,
    ) -> CasExecutionReport {
        CasExecutionReport::new(
            attempts_total,
            self.conflicts,
            self.retry_errors,
            self.aborts,
            self.timeouts,
            self.started_at,
            Instant::now(),
            max_attempts,
            max_operation_elapsed,
            max_total_elapsed,
            outcome,
        )
    }
}
