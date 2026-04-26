/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::time::Duration;

/// Human-readable profile for one CAS strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CasStrategyProfile {
    /// Maximum attempts used by the strategy.
    max_attempts: u32,
    /// Maximum total elapsed-time budget.
    max_elapsed: Duration,
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
        max_elapsed: Duration,
        target_conflict_ratio: f64,
        uses_backoff: bool,
    ) -> Self {
        Self {
            max_attempts,
            max_elapsed,
            target_conflict_ratio,
            uses_backoff,
        }
    }

    /// Returns the maximum attempts used by the strategy.
    ///
    /// # Returns
    /// Maximum number of attempts (including initial) for this strategy.
    #[inline]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Returns the maximum total elapsed-time budget.
    ///
    /// # Returns
    /// Total wall-clock time budget for the entire CAS flow.
    #[inline]
    pub fn max_elapsed(&self) -> Duration {
        self.max_elapsed
    }

    /// Returns the target conflict ratio for this strategy profile.
    ///
    /// # Returns
    /// The conflict ratio at which this strategy is recommended.
    #[inline]
    pub fn target_conflict_ratio(&self) -> f64 {
        self.target_conflict_ratio
    }

    /// Returns whether this profile uses retry backoff.
    ///
    /// # Returns
    /// `true` for strategies that insert delays between retries.
    #[inline]
    pub fn uses_backoff(&self) -> bool {
        self.uses_backoff
    }
}
