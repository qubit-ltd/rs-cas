// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Terminal fast compare-and-swap failures.

/// Terminal [`crate::FastCas`] failure.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FastCasError<E> {
    /// The operation aborted from a user decision.
    Abort {
        /// State observed by the aborting attempt.
        current: usize,

        /// Business error returned by the operation.
        error: E,

        /// Number of attempts consumed before aborting.
        attempts: u32,
    },

    /// CAS conflicts exhausted the retry policy.
    Conflict {
        /// Most recent state observed after the final failed CAS.
        current: usize,

        /// Number of attempts consumed before giving up.
        attempts: u32,
    },
}

impl<E> FastCasError<E> {
    /// Returns the current state captured by the terminal failure.
    ///
    /// # Returns
    /// The observed state code at failure.
    #[inline]
    pub const fn current(&self) -> usize {
        match self {
            Self::Abort { current, .. } | Self::Conflict { current, .. } => {
                *current
            }
        }
    }

    /// Returns the number of attempts consumed before failure.
    ///
    /// # Returns
    /// Number of attempts executed by the operation.
    #[inline]
    pub const fn attempts(&self) -> u32 {
        match self {
            Self::Abort { attempts, .. } | Self::Conflict { attempts, .. } => {
                *attempts
            }
        }
    }

    /// Tests whether this failure came from an abort decision.
    ///
    /// # Returns
    /// `true` for [`FastCasError::Abort`].
    #[inline]
    pub const fn is_abort(&self) -> bool {
        matches!(self, Self::Abort { .. })
    }

    /// Tests whether this failure came from exhausted CAS conflicts.
    ///
    /// # Returns
    /// `true` for [`FastCasError::Conflict`].
    #[inline]
    pub const fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// Returns the business error for abort failures.
    ///
    /// # Returns
    /// `Some(error)` for abort failures, or `None` for conflict failures.
    #[inline]
    pub const fn error(&self) -> Option<&E> {
        match self {
            Self::Abort { error, .. } => Some(error),
            Self::Conflict { .. } => None,
        }
    }

    /// Consumes this failure and returns the business error when present.
    ///
    /// # Returns
    /// `Some(error)` for abort failures, or `None` for conflict failures.
    #[inline]
    pub fn into_error(self) -> Option<E> {
        match self {
            Self::Abort { error, .. } => Some(error),
            Self::Conflict { .. } => None,
        }
    }
}
