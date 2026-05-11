/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Retry policy for fast compare-and-swap operations.

use konst::cmp::min;

/// Retry policy used by [`crate::FastCas`] when compare-and-swap loses a race.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FastCasPolicy {
    /// Try the operation once and return conflict on the first failed CAS.
    Once,

    /// Retry immediately until CAS succeeds, the operation aborts, or the
    /// attempt budget is exhausted.
    Spin {
        /// Maximum number of CAS attempts, including the first attempt.
        max_attempts: u32,
    },

    /// Spin for a short prefix, then yield between retries until the attempt
    /// budget is exhausted.
    SpinYield {
        /// Number of attempts to run without yielding.
        spin_attempts: u32,

        /// Maximum number of CAS attempts, including the first attempt.
        max_attempts: u32,
    },
}

impl FastCasPolicy {
    /// Creates a policy that performs one CAS attempt.
    ///
    /// # Returns
    /// A single-attempt policy.
    #[inline]
    pub const fn once() -> Self {
        Self::Once
    }

    /// Creates an immediate-spin retry policy.
    ///
    /// # Parameters
    /// - `max_attempts`: Maximum number of CAS attempts. Zero is normalized to
    ///   one attempt.
    ///
    /// # Returns
    /// A spin policy with a non-zero attempt budget.
    #[inline]
    pub const fn spin(max_attempts: u32) -> Self {
        Self::Spin {
            max_attempts: normalize_attempts(max_attempts),
        }
    }

    /// Creates a spin-then-yield retry policy.
    ///
    /// # Parameters
    /// - `spin_attempts`: Number of attempts to run before yielding.
    /// - `max_attempts`: Maximum number of CAS attempts. Zero is normalized to
    ///   one attempt.
    ///
    /// # Returns
    /// A spin-yield policy with a non-zero attempt budget.
    #[inline]
    pub const fn spin_yield(spin_attempts: u32, max_attempts: u32) -> Self {
        let max_attempts = normalize_attempts(max_attempts);
        Self::SpinYield {
            spin_attempts: min!(spin_attempts, max_attempts),
            max_attempts,
        }
    }

    /// Returns the maximum number of attempts allowed by this policy.
    ///
    /// # Returns
    /// A non-zero attempt count.
    #[inline]
    pub const fn max_attempts(self) -> u32 {
        match self {
            Self::Once => 1,
            Self::Spin { max_attempts } => normalize_attempts(max_attempts),
            Self::SpinYield { max_attempts, .. } => normalize_attempts(max_attempts),
        }
    }

    /// Returns whether the caller should yield before the next attempt.
    ///
    /// # Parameters
    /// - `next_attempt`: One-based number of the next attempt to execute.
    ///
    /// # Returns
    /// `true` when this policy should yield before `next_attempt`.
    #[inline]
    pub const fn should_yield_before(self, next_attempt: u32) -> bool {
        match self {
            Self::SpinYield { spin_attempts, .. } => next_attempt > spin_attempts,
            Self::Once | Self::Spin { .. } => false,
        }
    }
}

/// Normalizes an attempt budget so every policy can execute at least once.
///
/// # Parameters
/// - `attempts`: Requested attempt budget.
///
/// # Returns
/// `attempts` when non-zero, otherwise `1`.
#[inline]
const fn normalize_attempts(attempts: u32) -> u32 {
    if attempts == 0 { 1 } else { attempts }
}
