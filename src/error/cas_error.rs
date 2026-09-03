// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Terminal CAS errors.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use qubit_retry::AttemptFailure;
use qubit_retry::RetryContext;
use qubit_retry::RetryError;
use qubit_retry::RetryFailure;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryTimeoutScope;

use super::CasAttemptFailure;
use super::CasErrorKind;
use super::CasRetryFailure;
use super::internal::CasErrorDetails;
use crate::event::CasContext;

/// Terminal CAS error returned by [`crate::CasExecutor`].
#[derive(Clone)]
pub struct CasError<T, E> {
    /// Cached high-level CAS error kind.
    kind: CasErrorKind,
    /// Retry-layer details stored out of line to keep this error compact.
    details: Box<CasErrorDetails>,
    /// Retained application CAS failure or snapshot-backed timeout.
    last_failure: Option<CasAttemptFailure<T, E>>,
}

impl<T, E> CasError<T, E> {
    /// Wraps one retry-layer error.
    ///
    /// # Parameters
    /// - `inner`: Retry-layer error to wrap.
    /// - `timeout_current`: State snapshot captured before a retry-layer
    ///   timeout, when one is available.
    ///
    /// # Returns
    /// A [`CasError`] wrapper.
    #[inline]
    pub(crate) fn new(inner: RetryError<CasAttemptFailure<T, E>>, timeout_current: Option<Arc<T>>) -> Self {
        let (failure, retry_context) = inner.into_parts();
        Self::from_retry_parts(failure, retry_context, timeout_current)
    }

    /// Converts structured retry terminal parts into one CAS error.
    fn from_retry_parts(
        failure: RetryFailure<CasAttemptFailure<T, E>>,
        retry_context: RetryContext,
        mut timeout_current: Option<Arc<T>>,
    ) -> Self {
        let context = CasContext::new(&retry_context);
        let (failure, last_failure) = match failure {
            RetryFailure::Aborted { last_failure, .. } => (
                CasRetryFailure::Aborted,
                Self::map_attempt_failure(last_failure, &mut timeout_current),
            ),
            RetryFailure::Exhausted {
                limit, last_failure, ..
            } => (
                CasRetryFailure::Exhausted { limit },
                last_failure.and_then(|failure| Self::map_attempt_failure(failure, &mut timeout_current)),
            ),
            RetryFailure::TimedOut {
                scope, last_failure, ..
            } => (
                CasRetryFailure::TimedOut { scope },
                last_failure.and_then(|failure| Self::map_attempt_failure(failure, &mut timeout_current)),
            ),
            RetryFailure::Cancelled {
                phase, last_failure, ..
            } => (
                CasRetryFailure::Cancelled { phase },
                last_failure.and_then(|failure| Self::map_attempt_failure(failure, &mut timeout_current)),
            ),
            RetryFailure::CallbackFailed {
                callback, last_failure, ..
            } => (
                CasRetryFailure::CallbackFailed { callback },
                last_failure.and_then(|failure| Self::map_attempt_failure(failure, &mut timeout_current)),
            ),
            RetryFailure::Infrastructure {
                failure, last_failure, ..
            } => (
                CasRetryFailure::Infrastructure { failure },
                last_failure.and_then(|failure| Self::map_attempt_failure(failure, &mut timeout_current)),
            ),
            // Cargo.toml pins the published contract to exactly 0.19.0. A
            // substituted path source can nevertheless keep that package
            // version while adding a non-exhaustive variant, so degrade to a
            // safe structural terminal instead of panicking at runtime.
            _ => (CasRetryFailure::Unknown, None),
        };
        let kind = Self::classify_kind(&failure, last_failure.as_ref());
        Self {
            kind,
            details: Box::new(CasErrorDetails { failure, context }),
            last_failure,
        }
    }

    /// Extracts a CAS attempt failure only from an application attempt error.
    fn map_attempt_failure(
        failure: AttemptFailure<CasAttemptFailure<T, E>>,
        timeout_current: &mut Option<Arc<T>>,
    ) -> Option<CasAttemptFailure<T, E>> {
        match failure {
            AttemptFailure::Error(failure) => Some(failure),
            AttemptFailure::TimedOut { .. } => timeout_current.take().map(CasAttemptFailure::timeout),
            AttemptFailure::Panicked { .. } => None,
            _ => None,
        }
    }

    /// Returns the classified CAS error kind.
    ///
    /// # Returns
    /// High-level CAS error kind derived from the retry-layer reason and last
    /// attempt failure.
    #[must_use]
    #[inline(always)]
    pub fn kind(&self) -> CasErrorKind {
        self.kind
    }

    /// Returns the structured retry-layer terminal failure.
    ///
    /// # Returns
    /// Structured terminal classification and every detail available through
    /// the pinned retry-layer contract.
    #[must_use = "the structured retry terminal classification must be inspected"]
    #[inline(always)]
    pub fn failure(&self) -> &CasRetryFailure {
        &self.details.failure
    }

    /// Returns the terminal CAS context.
    ///
    /// # Returns
    /// Copied CAS context captured when execution stopped.
    #[must_use]
    #[inline(always)]
    pub fn context(&self) -> CasContext {
        self.details.context
    }

    /// Returns the number of attempts that were executed.
    ///
    /// # Returns
    /// One-based attempt count.
    #[must_use]
    #[inline(always)]
    pub fn attempts(&self) -> u32 {
        self.details.context.attempts()
    }

    /// Returns the retained application-level CAS failure, when one exists.
    ///
    /// # Returns
    /// `Some(&CasAttemptFailure<T, E>)` for a retained application CAS failure,
    /// including an asynchronous timeout only when a state snapshot was
    /// available. Returns `None` when the retry terminal retained no
    /// application failure or a timeout had no available state snapshot.
    #[must_use]
    #[inline(always)]
    pub fn last_failure(&self) -> Option<&CasAttemptFailure<T, E>> {
        self.last_failure.as_ref()
    }

    /// Consumes this error and returns the retained application-level CAS
    /// failure.
    ///
    /// # Returns
    /// `Some(CasAttemptFailure<T, E>)` for a retained application CAS failure,
    /// including an asynchronous timeout only when a state snapshot was
    /// available. Returns `None` when no application failure was retained;
    /// this is independent of whether attempts ran or infrastructure failed.
    #[must_use]
    #[inline(always)]
    pub fn into_last_failure(self) -> Option<CasAttemptFailure<T, E>> {
        self.last_failure
    }

    /// Consumes this error and returns all terminal error details.
    ///
    /// # Returns
    /// The classified kind, structured retry terminal failure, terminal
    /// context, and optional owned application CAS failure. The optional
    /// failure includes a timeout only when a state snapshot was available.
    #[must_use = "consuming the error returns its structured terminal details"]
    #[inline(always)]
    pub fn into_parts(
        self,
    ) -> (
        CasErrorKind,
        CasRetryFailure,
        CasContext,
        Option<CasAttemptFailure<T, E>>,
    ) {
        let CasErrorDetails { failure, context } = *self.details;
        (self.kind, failure, context, self.last_failure)
    }

    /// Returns the current state associated with the last failure.
    ///
    /// # Returns
    /// `Some(&Arc<T>)` when the terminal error preserved a current state.
    #[must_use]
    #[inline(always)]
    pub fn current(&self) -> Option<&Arc<T>> {
        self.last_failure().map(CasAttemptFailure::current)
    }

    /// Returns the business error associated with the last failure.
    ///
    /// # Returns
    /// `Some(&E)` for retryable or aborting business failures.
    #[must_use]
    #[inline(always)]
    pub fn error(&self) -> Option<&E> {
        self.last_failure().and_then(CasAttemptFailure::error)
    }

    /// Classifies one terminal CAS error kind from retry and attempt failures.
    ///
    /// # Parameters
    /// - `failure`: Structured terminal failure selected by the retry layer.
    /// - `last_failure`: Last CAS failure when one exists.
    ///
    /// # Returns
    /// Derived high-level CAS error kind.
    fn classify_kind(failure: &CasRetryFailure, last_failure: Option<&CasAttemptFailure<T, E>>) -> CasErrorKind {
        match failure {
            CasRetryFailure::Aborted => match last_failure {
                Some(CasAttemptFailure::Timeout { .. }) => CasErrorKind::AttemptTimeout,
                _ => CasErrorKind::Abort,
            },
            CasRetryFailure::Exhausted { limit } => match limit {
                RetryLimitKind::Attempts => match last_failure {
                    Some(CasAttemptFailure::Conflict { .. }) => CasErrorKind::Conflict,
                    Some(CasAttemptFailure::Timeout { .. }) => CasErrorKind::AttemptTimeout,
                    _ => CasErrorKind::RetryExhausted,
                },
                RetryLimitKind::OperationElapsed => CasErrorKind::MaxOperationElapsedExceeded,
                RetryLimitKind::TotalElapsed => CasErrorKind::MaxTotalElapsedExceeded,
            },
            CasRetryFailure::TimedOut { scope } => match scope {
                RetryTimeoutScope::Attempt => CasErrorKind::AttemptTimeout,
                RetryTimeoutScope::Flow => CasErrorKind::MaxTotalElapsedExceeded,
            },
            CasRetryFailure::Cancelled { .. }
            | CasRetryFailure::CallbackFailed { .. }
            | CasRetryFailure::Infrastructure { .. }
            | CasRetryFailure::Unknown => CasErrorKind::RetryInfrastructure,
        }
    }
}

impl<T, E> From<RetryError<CasAttemptFailure<T, E>>> for CasError<T, E> {
    /// Converts a retry error without an external async timeout snapshot.
    fn from(error: RetryError<CasAttemptFailure<T, E>>) -> Self {
        Self::new(error, None)
    }
}

impl<T, E> fmt::Debug for CasError<T, E> {
    /// Formats the CAS error for debugging without requiring `T: Debug`.
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
        f.debug_struct("CasError")
            .field("kind", &self.kind())
            .field("failure", &self.failure())
            .field("context", &self.context())
            .finish()
    }
}

impl<T, E> fmt::Display for CasError<T, E>
where
    E: fmt::Display,
{
    /// Formats the terminal CAS error.
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
        let message = match self.kind() {
            CasErrorKind::Abort => "CAS aborted",
            CasErrorKind::Conflict => "CAS conflicts exhausted",
            CasErrorKind::RetryExhausted => "CAS retryable failures exhausted",
            CasErrorKind::AttemptTimeout => "CAS attempt timed out",
            CasErrorKind::RetryInfrastructure => "CAS retry infrastructure failed",
            CasErrorKind::MaxOperationElapsedExceeded => "CAS max operation elapsed exceeded",
            CasErrorKind::MaxTotalElapsedExceeded => "CAS max total elapsed exceeded",
        };
        write!(f, "{message} after {} attempt(s)", self.attempts())?;
        write!(f, "; {}", self.failure())?;
        if let Some(failure) = self.last_failure() {
            write!(f, "; last failure: {failure}")?;
        }
        Ok(())
    }
}

impl<T, E> Error for CasError<T, E>
where
    E: Error + 'static,
{
    /// Returns the source business error when one exists.
    ///
    /// # Returns
    /// `Some(&dyn Error)` when the terminal CAS failure preserved a business
    /// error implementing [`std::error::Error`]. Callback panic payloads and
    /// infrastructure diagnostic strings do not fabricate error sources.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error().map(|error| error as &(dyn Error + 'static))
    }
}
