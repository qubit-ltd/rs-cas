// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Thresholds for classifying CAS contention.

/// Thresholds used to classify one execution as hotly contended.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicUsize;
/// use std::sync::atomic::Ordering;
///
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasAlert;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasHooks;
/// use qubit_cas::ContentionThresholds;
///
/// let state = AtomicRef::from_value(3usize);
/// let count = Arc::new(AtomicUsize::new(0));
/// let observed = Arc::clone(&count);
/// let thresholds = ContentionThresholds::new(1, 1, 1.0);
/// let hooks = CasHooks::new().on_contention_alert(thresholds, move |alert: &CasAlert| {
///     assert_eq!(alert.thresholds(), thresholds);
///     assert_eq!(alert.report().conflicts(), 1);
///     observed.fetch_add(1, Ordering::SeqCst);
/// });
/// let outcome = CasExecutor::<usize, ()>::builder().max_attempts(1).build().unwrap()
///     .execute_with_hooks(&state, |current: &usize| {
///         // Simulate another writer; avoid external side effects in real retry closures.
///         state.store(Arc::new(*current + 1));
///         CasDecision::update(*current + 2, ())
///     }, hooks);
/// assert!(outcome.is_err());
/// assert_eq!(count.load(Ordering::SeqCst), 1);
/// ```
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
    /// The conflict ratio is normalized to `[0.0, 1.0]`. `NaN` is normalized
    /// to `0.0` so threshold comparisons remain deterministic.
    ///
    /// # Parameters
    /// - `min_attempts`: Minimum attempts before ratio check is meaningful.
    /// - `min_conflicts`: Minimum absolute conflicts required.
    /// - `conflict_ratio`: Minimum conflict ratio normalized to `[0.0, 1.0]`.
    ///
    /// # Returns
    /// A normalized [`ContentionThresholds`] value.
    #[inline]
    #[must_use]
    pub fn new(min_attempts: u32, min_conflicts: u32, conflict_ratio: f64) -> Self {
        Self {
            min_attempts,
            min_conflicts,
            conflict_ratio: if conflict_ratio.is_nan() {
                0.0
            } else {
                conflict_ratio.clamp(0.0, 1.0)
            },
        }
    }

    /// Returns the minimum attempt count.
    ///
    /// # Returns
    /// Minimum number of attempts before a ratio is considered meaningful.
    #[must_use]
    #[inline(always)]
    pub fn min_attempts(&self) -> u32 {
        self.min_attempts
    }

    /// Returns the minimum conflict count.
    ///
    /// # Returns
    /// Minimum raw number of conflicts required to be considered hot.
    #[must_use]
    #[inline(always)]
    pub fn min_conflicts(&self) -> u32 {
        self.min_conflicts
    }

    /// Returns the minimum conflict ratio.
    ///
    /// # Returns
    /// Minimum ratio of conflicts to total attempts.
    #[must_use]
    #[inline(always)]
    pub fn conflict_ratio(&self) -> f64 {
        self.conflict_ratio
    }
}

impl Default for ContentionThresholds {
    /// Returns the recommended high-contention threshold.
    ///
    /// # Returns
    /// Thresholds requiring three attempts, one conflict, and at least 30%
    /// conflicts.
    #[inline(always)]
    fn default() -> Self {
        Self::new(3, 1, 0.30)
    }
}
