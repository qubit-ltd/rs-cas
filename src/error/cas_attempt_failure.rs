/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Attempt-level CAS failures.

use std::fmt;
use std::sync::Arc;

use super::cas_attempt_failure_kind::CasAttemptFailureKind;

/// Failure produced by one CAS attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CasAttemptFailure<T, E> {
    /// Compare-and-swap failed because another writer changed the state first.
    Conflict {
        /// State observed after the conflict.
        current: Arc<T>,
    },

    /// Business logic requested another attempt.
    Retry {
        /// Current state for the failed attempt.
        current: Arc<T>,
        /// Retryable business error.
        error: E,
    },

    /// Business logic aborted the CAS flow.
    Abort {
        /// Current state for the failed attempt.
        current: Arc<T>,
        /// Terminal business error.
        error: E,
    },

    /// One async attempt exceeded the configured timeout.
    Timeout {
        /// State snapshot used by the timed-out attempt.
        current: Arc<T>,
    },
}

impl<T, E> CasAttemptFailure<T, E> {
    /// Creates a conflict failure.
    ///
    /// # Parameters
    /// - `current`: Current state observed after the conflict.
    ///
    /// # Returns
    /// A [`CasAttemptFailure::Conflict`] value.
    #[inline]
    pub(crate) fn conflict(current: Arc<T>) -> Self {
        Self::Conflict { current }
    }

    /// Creates a retryable business failure.
    ///
    /// # Parameters
    /// - `current`: Current state for the failed attempt.
    /// - `error`: Retryable business error.
    ///
    /// # Returns
    /// A [`CasAttemptFailure::Retry`] value.
    #[inline]
    pub(crate) fn retry(current: Arc<T>, error: E) -> Self {
        Self::Retry { current, error }
    }

    /// Creates an abort failure.
    ///
    /// # Parameters
    /// - `current`: Current state for the failed attempt.
    /// - `error`: Terminal business error.
    ///
    /// # Returns
    /// A [`CasAttemptFailure::Abort`] value.
    #[inline]
    pub(crate) fn abort(current: Arc<T>, error: E) -> Self {
        Self::Abort { current, error }
    }

    /// Creates a timeout failure.
    ///
    /// # Parameters
    /// - `current`: State snapshot used by the timed-out attempt.
    ///
    /// # Returns
    /// A [`CasAttemptFailure::Timeout`] value.
    #[inline]
    #[cfg(feature = "tokio")]
    pub(crate) fn timeout(current: Arc<T>) -> Self {
        Self::Timeout { current }
    }

    /// Returns the state snapshot associated with this failure.
    ///
    /// # Returns
    /// Shared reference to the current state.
    #[inline]
    pub fn current(&self) -> &Arc<T> {
        match self {
            Self::Conflict { current }
            | Self::Retry { current, .. }
            | Self::Abort { current, .. }
            | Self::Timeout { current } => current,
        }
    }

    /// Returns the lightweight kind of this attempt failure.
    ///
    /// # Returns
    /// The [`CasAttemptFailureKind`] matching this failure variant.
    #[inline]
    pub fn kind(&self) -> CasAttemptFailureKind {
        match self {
            Self::Conflict { .. } => CasAttemptFailureKind::Conflict,
            Self::Retry { .. } => CasAttemptFailureKind::Retry,
            Self::Abort { .. } => CasAttemptFailureKind::Abort,
            Self::Timeout { .. } => CasAttemptFailureKind::Timeout,
        }
    }

    /// Returns the business error when this failure carries one.
    ///
    /// # Returns
    /// `Some(&E)` for [`CasAttemptFailure::Retry`] and
    /// [`CasAttemptFailure::Abort`], or `None` otherwise.
    #[inline]
    pub fn error(&self) -> Option<&E> {
        match self {
            Self::Retry { error, .. } | Self::Abort { error, .. } => Some(error),
            Self::Conflict { .. } | Self::Timeout { .. } => None,
        }
    }

    /// Returns whether this failure is a compare-and-swap conflict.
    ///
    /// # Returns
    /// `true` for [`CasAttemptFailure::Conflict`].
    #[inline]
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// Returns whether this failure is a retryable business error.
    ///
    /// # Returns
    /// `true` for [`CasAttemptFailure::Retry`].
    #[inline]
    pub fn is_retry(&self) -> bool {
        matches!(self, Self::Retry { .. })
    }

    /// Returns whether this failure aborts the CAS flow.
    ///
    /// # Returns
    /// `true` for [`CasAttemptFailure::Abort`].
    #[inline]
    pub fn is_abort(&self) -> bool {
        matches!(self, Self::Abort { .. })
    }

    /// Returns whether this failure came from an async timeout.
    ///
    /// # Returns
    /// `true` for [`CasAttemptFailure::Timeout`].
    #[inline]
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
}

impl<T, E> fmt::Display for CasAttemptFailure<T, E>
where
    E: fmt::Display,
{
    /// Formats the attempt failure for diagnostics.
    ///
    /// # Parameters
    /// - `f`: Formatter provided by the standard formatting machinery.
    ///
    /// # Returns
    /// `fmt::Result` from the formatter.
    ///
    /// # Errors
    /// Returns a formatting error if the formatter fails.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { .. } => write!(f, "compare-and-swap conflict"),
            Self::Retry { error, .. } => write!(f, "retryable CAS failure: {error}"),
            Self::Abort { error, .. } => write!(f, "aborted CAS failure: {error}"),
            Self::Timeout { .. } => write!(f, "CAS attempt timed out"),
        }
    }
}
