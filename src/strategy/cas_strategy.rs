// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Built-in CAS retry and latency strategies.

use std::time::Duration;

use super::CasStrategyProfile;
use crate::constants::CONTENTION_BACKOFF_INITIAL_DELAY;
use crate::constants::CONTENTION_BACKOFF_JITTER_FACTOR;
use crate::constants::CONTENTION_BACKOFF_MAX_ATTEMPTS;
use crate::constants::CONTENTION_BACKOFF_MAX_DELAY;
use crate::constants::CONTENTION_BACKOFF_MAX_ELAPSED;
use crate::constants::CONTENTION_BACKOFF_MAX_TOTAL_ELAPSED;
use crate::constants::LATENCY_FIRST_MAX_ATTEMPTS;
use crate::constants::LATENCY_FIRST_MAX_ELAPSED;
use crate::constants::LATENCY_FIRST_MAX_TOTAL_ELAPSED;
use crate::constants::RELIABILITY_FIRST_INITIAL_DELAY;
use crate::constants::RELIABILITY_FIRST_JITTER_FACTOR;
use crate::constants::RELIABILITY_FIRST_MAX_ATTEMPTS;
use crate::constants::RELIABILITY_FIRST_MAX_DELAY;
use crate::constants::RELIABILITY_FIRST_MAX_ELAPSED;
use crate::constants::RELIABILITY_FIRST_MAX_TOTAL_ELAPSED;

/// Built-in CAS execution strategy.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasStrategy {
    /// Optimizes for low latency with immediate retries and a smaller budget.
    LatencyFirst,
    /// Optimizes for hot contention with exponential backoff and jitter.
    ContentionBackoff,
    /// Optimizes for eventual success with a larger retry window.
    ReliabilityFirst,
}

impl Default for CasStrategy {
    /// Returns the default CAS strategy (`LatencyFirst`).
    ///
    /// # Returns
    /// [`CasStrategy::LatencyFirst`] as the recommended default.
    #[inline(always)]
    fn default() -> Self {
        Self::LatencyFirst
    }
}

impl CasStrategy {
    /// Returns the human-readable profile for this strategy.
    ///
    /// # Returns
    /// A [`CasStrategyProfile`] containing the parameters used by this
    /// strategy (max attempts, elapsed budgets, and backoff usage).
    #[inline]
    #[must_use]
    pub fn profile(self) -> CasStrategyProfile {
        match self {
            Self::LatencyFirst => CasStrategyProfile::new(
                LATENCY_FIRST_MAX_ATTEMPTS,
                LATENCY_FIRST_MAX_ELAPSED,
                Some(LATENCY_FIRST_MAX_TOTAL_ELAPSED),
                false,
            ),
            Self::ContentionBackoff => CasStrategyProfile::new(
                CONTENTION_BACKOFF_MAX_ATTEMPTS,
                CONTENTION_BACKOFF_MAX_ELAPSED,
                Some(CONTENTION_BACKOFF_MAX_TOTAL_ELAPSED),
                true,
            ),
            Self::ReliabilityFirst => CasStrategyProfile::new(
                RELIABILITY_FIRST_MAX_ATTEMPTS,
                RELIABILITY_FIRST_MAX_ELAPSED,
                Some(RELIABILITY_FIRST_MAX_TOTAL_ELAPSED),
                true,
            ),
        }
    }

    /// Returns retry delay settings for strategies that use backoff.
    ///
    /// # Returns
    /// `Some((initial, max, jitter_factor))` for backoff strategies, or `None`
    /// for latency-first (immediate retries).
    #[must_use]
    #[inline]
    pub(crate) fn backoff(self) -> Option<(Duration, Duration, f64)> {
        match self {
            Self::LatencyFirst => None,
            Self::ContentionBackoff => Some((
                CONTENTION_BACKOFF_INITIAL_DELAY,
                CONTENTION_BACKOFF_MAX_DELAY,
                CONTENTION_BACKOFF_JITTER_FACTOR,
            )),
            Self::ReliabilityFirst => Some((
                RELIABILITY_FIRST_INITIAL_DELAY,
                RELIABILITY_FIRST_MAX_DELAY,
                RELIABILITY_FIRST_JITTER_FACTOR,
            )),
        }
    }
}
