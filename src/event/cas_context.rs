/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
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
    /// Configured maximum total elapsed time.
    max_elapsed: Option<Duration>,
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
    /// Creates a context from a retry context plus the configured timeout.
    ///
    /// # Parameters
    /// - `context`: Retry-layer context to copy.
    /// - `attempt_timeout`: CAS attempt timeout configured by the executor.
    ///
    /// # Returns
    /// A copied [`CasContext`] value.
    #[inline]
    pub(crate) fn new(
        context: &RetryContext,
        attempt_timeout: Option<Duration>,
    ) -> Self {
        Self {
            attempt: context.attempt(),
            max_attempts: context.max_attempts(),
            max_elapsed: context.max_elapsed(),
            total_elapsed: context.total_elapsed(),
            attempt_elapsed: context.attempt_elapsed(),
            attempt_timeout: attempt_timeout.or(context.attempt_timeout()),
            next_delay: context.next_delay(),
        }
    }

    /// Returns the current attempt number.
    ///
    /// # Returns
    /// A one-based attempt number, or zero if no attempt ran.
    #[inline]
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns the configured maximum attempts.
    ///
    /// # Returns
    /// Maximum attempts, including the initial attempt.
    #[inline]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the configured maximum retries after the initial attempt.
    ///
    /// # Returns
    /// Maximum retries after the first attempt.
    #[inline]
    pub fn max_retries(&self) -> u32 {
        self.max_attempts.saturating_sub(1)
    }

    /// Returns the maximum elapsed-time budget.
    ///
    /// # Returns
    /// `Some(Duration)` for bounded executions, or `None` for unlimited.
    #[inline]
    pub fn max_elapsed(&self) -> Option<Duration> {
        self.max_elapsed
    }

    /// Returns elapsed time since the CAS flow started.
    ///
    /// # Returns
    /// Total elapsed time observed at this event.
    #[inline]
    pub fn total_elapsed(&self) -> Duration {
        self.total_elapsed
    }

    /// Returns elapsed time spent in the current attempt.
    ///
    /// # Returns
    /// Attempt elapsed time.
    #[inline]
    pub fn attempt_elapsed(&self) -> Duration {
        self.attempt_elapsed
    }

    /// Returns the configured timeout for each async attempt.
    ///
    /// # Returns
    /// `Some(Duration)` when an attempt timeout is configured.
    #[inline]
    pub fn attempt_timeout(&self) -> Option<Duration> {
        self.attempt_timeout
    }

    /// Returns the selected delay before the next retry.
    ///
    /// # Returns
    /// `Some(Duration)` when retry scheduling selected a delay.
    #[inline]
    pub fn next_delay(&self) -> Option<Duration> {
        self.next_delay
    }
}
