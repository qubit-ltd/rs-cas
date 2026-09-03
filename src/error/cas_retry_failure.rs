// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Structured retry-layer terminal failures.

use std::fmt;

use qubit_retry::RetryCallbackFailure;
use qubit_retry::RetryCancellationPhase;
use qubit_retry::RetryInfrastructureFailure;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryTimeoutScope;

/// Structured retry-layer terminal classification retained by
/// [`crate::CasError`].
///
/// Attempt-level CAS failures are stored separately on [`crate::CasError`].
/// The six `qubit-retry` 0.19.0 terminals preserve their exact details without
/// reclassifying callback or infrastructure failures as CAS business errors.
/// [`Self::Unknown`] safely contains a substituted path source that extends
/// the pinned upstream terminal enum without changing its package metadata.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CasRetryFailure {
    /// A retry rule deliberately stopped the flow.
    Aborted,
    /// A continuation limit prevented another attempt.
    Exhausted {
        /// Exact limit that was exhausted.
        limit: RetryLimitKind,
    },
    /// A hard timeout stopped execution.
    TimedOut {
        /// Scope whose timeout expired.
        scope: RetryTimeoutScope,
    },
    /// External cancellation stopped execution.
    Cancelled {
        /// Retry phase in which cancellation was observed.
        phase: RetryCancellationPhase,
    },
    /// A retry rule or observer panicked.
    CallbackFailed {
        /// Exact callback kind, index, phase, and panic payload.
        callback: RetryCallbackFailure,
    },
    /// Retry infrastructure could not continue safely.
    Infrastructure {
        /// Exact clock, timer, worker-spawn, or worker-stop failure.
        failure: RetryInfrastructureFailure,
    },
    /// A terminal variant outside the exact `qubit-retry` 0.19.0 contract.
    ///
    /// This defensive classification can only be observed when a substituted
    /// path source keeps the 0.19.0 package metadata while extending its
    /// non-exhaustive terminal enum. The unknown variant cannot expose owned
    /// terminal fields that the pinned API does not define.
    Unknown,
}

impl CasRetryFailure {
    /// Returns the exhausted continuation limit, when applicable.
    ///
    /// # Returns
    /// `Some(RetryLimitKind)` for [`Self::Exhausted`], or `None` for every
    /// other terminal classification.
    #[inline(always)]
    #[must_use]
    pub fn limit(&self) -> Option<RetryLimitKind> {
        match self {
            Self::Exhausted { limit } => Some(*limit),
            _ => None,
        }
    }

    /// Returns the hard-timeout scope, when applicable.
    ///
    /// # Returns
    /// `Some(RetryTimeoutScope)` for [`Self::TimedOut`], or `None` otherwise.
    #[inline(always)]
    #[must_use]
    pub fn timeout_scope(&self) -> Option<RetryTimeoutScope> {
        match self {
            Self::TimedOut { scope } => Some(*scope),
            _ => None,
        }
    }

    /// Returns the cancellation phase, when applicable.
    ///
    /// # Returns
    /// `Some(RetryCancellationPhase)` for [`Self::Cancelled`], or `None`
    /// otherwise.
    #[inline(always)]
    #[must_use]
    pub fn cancellation_phase(&self) -> Option<RetryCancellationPhase> {
        match self {
            Self::Cancelled { phase } => Some(*phase),
            _ => None,
        }
    }

    /// Returns the callback failure attribution, when applicable.
    ///
    /// # Returns
    /// `Some(&RetryCallbackFailure)` for [`Self::CallbackFailed`], or
    /// `None` otherwise.
    #[inline(always)]
    #[must_use]
    pub fn callback_failure(&self) -> Option<&RetryCallbackFailure> {
        match self {
            Self::CallbackFailed { callback } => Some(callback),
            _ => None,
        }
    }

    /// Returns the infrastructure failure, when applicable.
    ///
    /// # Returns
    /// `Some(&RetryInfrastructureFailure)` for [`Self::Infrastructure`], or
    /// `None` otherwise.
    #[inline(always)]
    #[must_use]
    pub fn infrastructure_failure(&self) -> Option<&RetryInfrastructureFailure> {
        match self {
            Self::Infrastructure { failure } => Some(failure),
            _ => None,
        }
    }
}

impl fmt::Display for CasRetryFailure {
    /// Formats the structured terminal classification.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aborted => formatter.write_str("retry aborted"),
            Self::Exhausted { limit } => {
                write!(formatter, "retry limit exhausted: {limit}")
            }
            Self::TimedOut { scope } => {
                write!(formatter, "retry timed out: {scope}")
            }
            Self::Cancelled { phase } => {
                write!(formatter, "retry cancelled: {phase}")
            }
            Self::CallbackFailed { callback } => {
                write!(formatter, "retry callback failed: {callback}")
            }
            Self::Infrastructure { failure } => {
                write!(formatter, "retry infrastructure failed: {failure}")
            }
            Self::Unknown => formatter.write_str("unknown retry terminal failure"),
        }
    }
}
