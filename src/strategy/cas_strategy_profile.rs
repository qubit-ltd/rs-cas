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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CasStrategyProfile {
    /// Maximum attempts used by the strategy.
    max_attempts: u32,
    /// Maximum cumulative user operation elapsed-time budget.
    max_operation_elapsed: Duration,
    /// Optional monotonic total retry-flow elapsed-time budget.
    max_total_elapsed: Option<Duration>,
    /// Conflict ratio at which callers should consider this profile suitable.
    target_conflict_ratio: f64,
    /// Whether the strategy uses retry backoff.
    uses_backoff: bool,
}

impl CasStrategyProfile {
    /// Creates a strategy profile from static preset values.
    #[inline]
    pub(crate) const fn new(
        max_attempts: u32,
        max_operation_elapsed: Duration,
        max_total_elapsed: Option<Duration>,
        target_conflict_ratio: f64,
        uses_backoff: bool,
    ) -> Self {
        Self {
            max_attempts,
            max_operation_elapsed,
            max_total_elapsed,
            target_conflict_ratio,
            uses_backoff,
        }
    }

    /// Returns the maximum attempts used by the strategy.
    ///
    /// # Returns
    /// Maximum number of attempts (including initial) for this strategy.
    #[inline(always)]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the maximum cumulative user operation elapsed-time budget.
    ///
    /// # Returns
    /// User operation time budget for the entire CAS flow.
    #[inline(always)]
    pub fn max_operation_elapsed(&self) -> Duration {
        self.max_operation_elapsed
    }

    /// Returns the optional monotonic total retry-flow elapsed-time budget.
    ///
    /// # Returns
    /// `Some(Duration)` when the strategy caps whole-flow time (including retry
    /// sleeps), or `None` when only the operation-time budget applies.
    #[inline(always)]
    pub fn max_total_elapsed(&self) -> Option<Duration> {
        self.max_total_elapsed
    }

    /// Returns the target conflict ratio for this strategy profile.
    ///
    /// # Returns
    /// The conflict ratio at which this strategy is recommended.
    #[inline(always)]
    pub fn target_conflict_ratio(&self) -> f64 {
        self.target_conflict_ratio
    }

    /// Returns whether this profile uses retry backoff.
    ///
    /// # Returns
    /// `true` for strategies that insert delays between retries.
    #[inline(always)]
    pub fn uses_backoff(&self) -> bool {
        self.uses_backoff
    }
}
