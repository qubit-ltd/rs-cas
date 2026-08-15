// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS execution context payload.

use std::num::NonZeroU32;
use std::time::Duration;

use qubit_retry::RetryContext;

/// Context captured for CAS lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CasContext {
    /// Number of operations that actually started.
    attempts: u32,
    /// Attempt associated with the current retry event, when present.
    current_attempt: Option<NonZeroU32>,
    /// Configured maximum attempts.
    max_attempts: u32,
    /// Configured maximum cumulative user operation time.
    max_operation_elapsed: Option<Duration>,
    /// Configured maximum total retry-flow elapsed time.
    max_total_elapsed: Option<Duration>,
    /// Elapsed time since the CAS flow started.
    total_elapsed: Duration,
    /// Time spent in the latest completed attempt.
    last_attempt_elapsed: Duration,
    /// Effective timeout selected for the current async attempt.
    current_attempt_timeout: Option<Duration>,
    /// Delay selected before the next retry, when known.
    next_delay: Option<Duration>,
}

impl CasContext {
    /// Creates a context from a retry context.
    ///
    /// # Parameters
    /// - `context`: Retry-layer context to copy.
    ///
    /// # Returns
    /// A copied [`CasContext`] value.
    #[inline]
    pub(crate) fn new(context: &RetryContext) -> Self {
        Self {
            attempts: context.attempts(),
            current_attempt: context.current_attempt(),
            max_attempts: context.max_attempts(),
            max_operation_elapsed: context.max_operation_elapsed(),
            max_total_elapsed: context.max_total_elapsed(),
            total_elapsed: context.total_elapsed(),
            last_attempt_elapsed: context.last_attempt_elapsed(),
            current_attempt_timeout: context.current_attempt_timeout(),
            next_delay: context.next_delay(),
        }
    }

    /// Returns the number of operations that actually started.
    ///
    /// # Returns
    /// The committed operation count, or zero if no operation ran.
    #[must_use]
    #[inline(always)]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Returns the attempt associated with the current retry event.
    ///
    /// # Returns
    /// `Some(NonZeroU32)` for attempt-related contexts, or `None` for terminal
    /// contexts without a current attempt.
    #[must_use]
    #[inline(always)]
    pub fn current_attempt(&self) -> Option<NonZeroU32> {
        self.current_attempt
    }

    /// Returns the configured maximum attempts.
    ///
    /// # Returns
    /// Maximum attempts, including the initial attempt.
    #[must_use]
    #[inline(always)]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the configured maximum retries after the initial attempt.
    ///
    /// # Returns
    /// Maximum retries after the first attempt.
    #[must_use]
    #[inline(always)]
    pub fn max_retries(&self) -> u32 {
        self.max_attempts.saturating_sub(1)
    }

    /// Returns the configured maximum cumulative user operation time budget.
    ///
    /// # Returns
    /// `Some(Duration)` for bounded executions, or `None` for unlimited.
    #[must_use]
    #[inline(always)]
    pub fn max_operation_elapsed(&self) -> Option<Duration> {
        self.max_operation_elapsed
    }

    /// Returns the configured maximum total retry-flow elapsed-time budget.
    ///
    /// # Returns
    /// `Some(Duration)` for bounded executions, or `None` for unlimited.
    #[must_use]
    #[inline(always)]
    pub fn max_total_elapsed(&self) -> Option<Duration> {
        self.max_total_elapsed
    }

    /// Returns elapsed time since the CAS flow started.
    ///
    /// # Returns
    /// Total elapsed time observed at this event.
    #[must_use]
    #[inline(always)]
    pub fn total_elapsed(&self) -> Duration {
        self.total_elapsed
    }

    /// Returns elapsed time spent in the latest completed attempt.
    ///
    /// # Returns
    /// Latest completed attempt elapsed time, or zero before one completes.
    #[must_use]
    #[inline(always)]
    pub fn last_attempt_elapsed(&self) -> Duration {
        self.last_attempt_elapsed
    }

    /// Returns the effective timeout for the current async attempt.
    ///
    /// # Returns
    /// `Some(Duration)` when the current attempt has a hard timeout.
    #[must_use]
    #[inline(always)]
    pub fn current_attempt_timeout(&self) -> Option<Duration> {
        self.current_attempt_timeout
    }

    /// Returns the selected delay before the next retry.
    ///
    /// # Returns
    /// `Some(Duration)` when retry scheduling selected a delay.
    #[must_use]
    #[inline(always)]
    pub fn next_delay(&self) -> Option<Duration> {
        self.next_delay
    }
}
