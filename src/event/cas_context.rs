// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS execution context payload.

use std::time::Duration;

use qubit_retry::RetryContext;

/// Context captured for CAS lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CasContext {
    /// Current attempt number, or zero if no attempt ran.
    attempt: u32,
    /// Configured maximum attempts.
    max_attempts: u32,
    /// Configured maximum cumulative user operation time.
    max_operation_elapsed: Option<Duration>,
    /// Configured maximum total retry-flow elapsed time.
    max_total_elapsed: Option<Duration>,
    /// Elapsed time since the CAS flow started.
    total_elapsed: Duration,
    /// Time spent in the current attempt.
    attempt_elapsed: Duration,
    /// Optional timeout configured for each async attempt.
    attempt_timeout: Option<Duration>,
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
            attempt: context.attempt(),
            max_attempts: context.max_attempts(),
            max_operation_elapsed: context.max_operation_elapsed(),
            max_total_elapsed: context.max_total_elapsed(),
            total_elapsed: context.total_elapsed(),
            attempt_elapsed: context.attempt_elapsed(),
            attempt_timeout: context.attempt_timeout(),
            next_delay: context.next_delay(),
        }
    }

    /// Returns the current attempt number.
    ///
    /// # Returns
    /// A one-based attempt number, or zero if no attempt ran.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns the configured maximum attempts.
    ///
    /// # Returns
    /// Maximum attempts, including the initial attempt.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the configured maximum retries after the initial attempt.
    ///
    /// # Returns
    /// Maximum retries after the first attempt.
    pub fn max_retries(&self) -> u32 {
        self.max_attempts.saturating_sub(1)
    }

    /// Returns the configured maximum cumulative user operation time budget.
    ///
    /// # Returns
    /// `Some(Duration)` for bounded executions, or `None` for unlimited.
    pub fn max_operation_elapsed(&self) -> Option<Duration> {
        self.max_operation_elapsed
    }

    /// Returns the configured maximum total retry-flow elapsed-time budget.
    ///
    /// # Returns
    /// `Some(Duration)` for bounded executions, or `None` for unlimited.
    pub fn max_total_elapsed(&self) -> Option<Duration> {
        self.max_total_elapsed
    }

    /// Returns elapsed time since the CAS flow started.
    ///
    /// # Returns
    /// Total elapsed time observed at this event.
    pub fn total_elapsed(&self) -> Duration {
        self.total_elapsed
    }

    /// Returns elapsed time spent in the current attempt.
    ///
    /// # Returns
    /// Attempt elapsed time.
    pub fn attempt_elapsed(&self) -> Duration {
        self.attempt_elapsed
    }

    /// Returns the configured timeout for each async attempt.
    ///
    /// # Returns
    /// `Some(Duration)` when an attempt timeout is configured.
    pub fn attempt_timeout(&self) -> Option<Duration> {
        self.attempt_timeout
    }

    /// Returns the selected delay before the next retry.
    ///
    /// # Returns
    /// `Some(Duration)` when retry scheduling selected a delay.
    pub fn next_delay(&self) -> Option<Duration> {
        self.next_delay
    }
}
