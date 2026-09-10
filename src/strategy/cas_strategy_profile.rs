// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Descriptive values exposed by a built-in CAS strategy.

use std::time::Duration;

/// Human-readable profile for one CAS strategy.
///
/// # Examples
///
/// ```
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasStrategy;
///
/// let strategy = CasStrategy::ReliabilityFirst;
/// let profile = strategy.profile();
/// let executor = CasExecutor::<usize, ()>::with_strategy(strategy);
/// assert_eq!(executor.max_attempts(), profile.max_attempts());
/// assert_eq!(executor.max_operation_elapsed(), Some(profile.max_operation_elapsed()));
/// assert!(profile.uses_backoff());
/// let custom = CasExecutor::<usize, ()>::builder().strategy(strategy).max_attempts(2).build().unwrap();
/// assert_eq!(custom.max_attempts(), 2);
/// assert_ne!(custom.max_attempts(), profile.max_attempts());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CasStrategyProfile {
    /// Maximum attempts used by the strategy.
    max_attempts: u32,
    /// Maximum cumulative attempt elapsed-time budget.
    max_operation_elapsed: Duration,
    /// Optional monotonic total retry-flow elapsed-time budget.
    max_total_elapsed: Option<Duration>,
    /// Whether the strategy uses retry backoff.
    uses_backoff: bool,
}

impl CasStrategyProfile {
    /// Creates a strategy profile from static preset values.
    ///
    /// # Parameters
    /// - `max_attempts`: Preset attempt count including the first operation.
    /// - `max_operation_elapsed`: Cumulative attempt budget, including adapter
    ///   work.
    /// - `max_total_elapsed`: `Some` soft total budget, or `None` if disabled.
    /// - `uses_backoff`: Whether the preset inserts delays between attempts.
    ///
    /// # Returns
    /// A descriptive preset; executor setters may subsequently override its
    /// values.
    #[inline]
    #[must_use]
    pub(crate) const fn new(
        max_attempts: u32,
        max_operation_elapsed: Duration,
        max_total_elapsed: Option<Duration>,
        uses_backoff: bool,
    ) -> Self {
        Self {
            max_attempts,
            max_operation_elapsed,
            max_total_elapsed,
            uses_backoff,
        }
    }

    /// Returns the maximum attempts used by the strategy.
    ///
    /// # Returns
    /// Maximum number of attempts (including initial) for this strategy.
    #[must_use]
    #[inline(always)]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the maximum cumulative attempt elapsed-time budget.
    ///
    /// # Returns
    /// Soft admission budget for accumulated operation and CAS adapter work.
    #[must_use]
    #[inline(always)]
    pub fn max_operation_elapsed(&self) -> Duration {
        self.max_operation_elapsed
    }

    /// Returns the optional monotonic total retry-flow elapsed-time budget.
    ///
    /// # Returns
    /// `Some(Duration)` enables soft admission checks for whole-flow time,
    /// including retry sleeps; `None` disables this budget. An admitted
    /// operation can still complete after the budget expires.
    #[must_use]
    #[inline(always)]
    pub fn max_total_elapsed(&self) -> Option<Duration> {
        self.max_total_elapsed
    }

    /// Returns whether this profile uses retry backoff.
    ///
    /// # Returns
    /// `true` for strategies that insert delays between retries.
    #[must_use]
    #[inline(always)]
    pub fn uses_backoff(&self) -> bool {
        self.uses_backoff
    }
}
