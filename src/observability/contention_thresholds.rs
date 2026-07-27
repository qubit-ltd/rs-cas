// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Thresholds for classifying CAS contention.

/// Thresholds used to classify one execution as hotly contended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentionThresholds {
    /// Minimum attempt count before the ratio is meaningful.
    min_attempts: u32,
    /// Minimum raw conflict count.
    min_conflicts: u32,
    /// Minimum conflict ratio.
    conflict_ratio: f64,
}

impl ContentionThresholds {
    /// Creates contention thresholds from raw values.
    ///
    /// The conflict ratio is clamped to `[0.0, 1.0]`.
    ///
    /// # Parameters
    /// - `min_attempts`: Minimum attempts before ratio check is meaningful.
    /// - `min_conflicts`: Minimum absolute conflicts required.
    /// - `conflict_ratio`: Minimum conflict ratio (clamped to [0.0, 1.0]).
    ///
    /// # Returns
    /// A normalized [`ContentionThresholds`] value.
    #[inline]
    pub fn new(min_attempts: u32, min_conflicts: u32, conflict_ratio: f64) -> Self {
        Self {
            min_attempts,
            min_conflicts,
            conflict_ratio: conflict_ratio.clamp(0.0, 1.0),
        }
    }

    /// Returns the minimum attempt count.
    ///
    /// # Returns
    /// Minimum number of attempts before a ratio is considered meaningful.
    #[inline(always)]
    pub fn min_attempts(&self) -> u32 {
        self.min_attempts
    }

    /// Returns the minimum conflict count.
    ///
    /// # Returns
    /// Minimum raw number of conflicts required to be considered hot.
    #[inline(always)]
    pub fn min_conflicts(&self) -> u32 {
        self.min_conflicts
    }

    /// Returns the minimum conflict ratio.
    ///
    /// # Returns
    /// Minimum ratio of conflicts to total attempts.
    #[inline(always)]
    pub fn conflict_ratio(&self) -> f64 {
        self.conflict_ratio
    }
}

impl Default for ContentionThresholds {
    /// Returns the recommended high-contention threshold.
    #[inline]
    fn default() -> Self {
        Self::new(3, 1, 0.30)
    }
}
