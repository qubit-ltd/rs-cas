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
use crate::constants::CONTENTION_ADAPTIVE_INITIAL_DELAY;
use crate::constants::CONTENTION_ADAPTIVE_JITTER_FACTOR;
use crate::constants::CONTENTION_ADAPTIVE_MAX_ATTEMPTS;
use crate::constants::CONTENTION_ADAPTIVE_MAX_DELAY;
use crate::constants::CONTENTION_ADAPTIVE_MAX_ELAPSED;
use crate::constants::CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasStrategy {
    /// Optimizes for low latency with immediate retries and a smaller budget.
    LatencyFirst,
    /// Optimizes for hot contention with exponential backoff and jitter.
    ContentionAdaptive,
    /// Optimizes for eventual success with a larger retry window.
    ReliabilityFirst,
}

impl Default for CasStrategy {
    /// Returns the default CAS strategy (`LatencyFirst`).
    ///
    /// # Returns
    /// [`CasStrategy::LatencyFirst`] as the recommended default.
    #[inline]
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
    pub fn profile(self) -> CasStrategyProfile {
        match self {
            Self::LatencyFirst => CasStrategyProfile::new(
                LATENCY_FIRST_MAX_ATTEMPTS,
                LATENCY_FIRST_MAX_ELAPSED,
                Some(LATENCY_FIRST_MAX_TOTAL_ELAPSED),
                false,
            ),
            Self::ContentionAdaptive => CasStrategyProfile::new(
                CONTENTION_ADAPTIVE_MAX_ATTEMPTS,
                CONTENTION_ADAPTIVE_MAX_ELAPSED,
                Some(CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED),
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
    #[inline]
    pub(crate) fn backoff(self) -> Option<(Duration, Duration, f64)> {
        match self {
            Self::LatencyFirst => None,
            Self::ContentionAdaptive => Some((
                CONTENTION_ADAPTIVE_INITIAL_DELAY,
                CONTENTION_ADAPTIVE_MAX_DELAY,
                CONTENTION_ADAPTIVE_JITTER_FACTOR,
            )),
            Self::ReliabilityFirst => Some((
                RELIABILITY_FIRST_INITIAL_DELAY,
                RELIABILITY_FIRST_MAX_DELAY,
                RELIABILITY_FIRST_JITTER_FACTOR,
            )),
        }
    }
}
