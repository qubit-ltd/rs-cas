/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Terminal CAS errors.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use qubit_retry::{AttemptFailure, RetryError, RetryErrorReason};

use crate::event::CasContext;

use super::CasAttemptFailure;

/// Classified reason for a terminal CAS error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasErrorKind {
    /// The operation explicitly aborted.
    Abort,
    /// Retry limits were exhausted by compare-and-swap conflicts.
    Conflict,
    /// Retry limits were exhausted by retryable business failures.
    RetryExhausted,
    /// A timeout aborted the flow or exhausted retry limits.
    AttemptTimeout,
    /// The total elapsed-time budget expired.
    MaxElapsedExceeded,
}

/// Terminal CAS error returned by [`crate::CasExecutor`].
#[derive(Clone)]
pub struct CasError<T, E> {
    /// Underlying retry-layer error.
    inner: Box<RetryError<CasAttemptFailure<T, E>>>,
    /// Optional async attempt timeout configured by the executor.
    attempt_timeout: Option<std::time::Duration>,
}

impl<T, E> fmt::Debug for CasError<T, E>
where
    E: fmt::Display,
{
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
            .field("reason", &self.reason())
            .field("context", &self.context())
            .field("attempt_timeout", &self.attempt_timeout)
            .finish()
    }
}

impl<T, E> CasError<T, E> {
    /// Wraps one retry-layer error.
    ///
    /// # Parameters
    /// - `inner`: Retry-layer error to wrap.
    /// - `attempt_timeout`: Optional timeout configured by the executor.
    ///
    /// # Returns
    /// A [`CasError`] wrapper.
    #[inline]
    pub(crate) fn new(
        inner: RetryError<CasAttemptFailure<T, E>>,
        attempt_timeout: Option<std::time::Duration>,
    ) -> Self {
        Self {
            inner: Box::new(inner),
            attempt_timeout,
        }
    }

    /// Returns the classified CAS error kind.
    ///
    /// # Returns
    /// High-level CAS error kind derived from the retry-layer reason and last
    /// attempt failure.
    pub fn kind(&self) -> CasErrorKind {
        match self.inner.reason() {
            RetryErrorReason::Aborted => match self.last_failure() {
                Some(CasAttemptFailure::Timeout { .. }) => CasErrorKind::AttemptTimeout,
                Some(CasAttemptFailure::Abort { .. }) | None => CasErrorKind::Abort,
                Some(CasAttemptFailure::Conflict { .. })
                | Some(CasAttemptFailure::Retry { .. }) => CasErrorKind::Abort,
            },
            RetryErrorReason::AttemptsExceeded => match self.last_failure() {
                Some(CasAttemptFailure::Conflict { .. }) => CasErrorKind::Conflict,
                Some(CasAttemptFailure::Retry { .. }) | None => CasErrorKind::RetryExhausted,
                Some(CasAttemptFailure::Timeout { .. }) => CasErrorKind::AttemptTimeout,
                Some(CasAttemptFailure::Abort { .. }) => CasErrorKind::Abort,
            },
            RetryErrorReason::MaxElapsedExceeded => CasErrorKind::MaxElapsedExceeded,
        }
    }

    /// Returns the retry-layer terminal reason.
    ///
    /// # Returns
    /// Underlying [`RetryErrorReason`].
    #[inline]
    pub fn reason(&self) -> RetryErrorReason {
        self.inner.reason()
    }

    /// Returns the terminal CAS context.
    ///
    /// # Returns
    /// Copied CAS context captured when execution stopped.
    #[inline]
    pub fn context(&self) -> CasContext {
        CasContext::from_retry_context(self.inner.context(), self.attempt_timeout)
    }

    /// Returns the number of attempts that were executed.
    ///
    /// # Returns
    /// One-based attempt count.
    #[inline]
    pub fn attempts(&self) -> u32 {
        self.inner.attempts()
    }

    /// Returns the last CAS attempt failure when one exists.
    ///
    /// # Returns
    /// `Some(&CasAttemptFailure<T, E>)` when at least one attempt failed.
    #[inline]
    pub fn last_failure(&self) -> Option<&CasAttemptFailure<T, E>> {
        match self.inner.last_failure() {
            Some(AttemptFailure::Error(failure)) => Some(failure),
            Some(AttemptFailure::Timeout) | None => None,
        }
    }

    /// Returns the current state associated with the last failure.
    ///
    /// # Returns
    /// `Some(&Arc<T>)` when the terminal error preserved a current state.
    #[inline]
    pub fn current(&self) -> Option<&Arc<T>> {
        self.last_failure().map(CasAttemptFailure::current)
    }

    /// Returns the business error associated with the last failure.
    ///
    /// # Returns
    /// `Some(&E)` for retryable or aborting business failures.
    #[inline]
    pub fn error(&self) -> Option<&E> {
        self.last_failure().and_then(CasAttemptFailure::error)
    }

    /// Consumes the wrapper and returns the underlying retry error.
    ///
    /// # Returns
    /// Owned retry-layer error.
    #[inline]
    pub fn into_inner(self) -> RetryError<CasAttemptFailure<T, E>> {
        *self.inner
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
        match self.kind() {
            CasErrorKind::Abort => write!(f, "CAS aborted after {} attempt(s)", self.attempts())?,
            CasErrorKind::Conflict => write!(
                f,
                "CAS conflicts exhausted after {} attempt(s)",
                self.attempts()
            )?,
            CasErrorKind::RetryExhausted => write!(
                f,
                "CAS retryable failures exhausted after {} attempt(s)",
                self.attempts()
            )?,
            CasErrorKind::AttemptTimeout => write!(
                f,
                "CAS attempt timed out after {} attempt(s)",
                self.attempts()
            )?,
            CasErrorKind::MaxElapsedExceeded => write!(
                f,
                "CAS max elapsed exceeded after {} attempt(s)",
                self.attempts()
            )?,
        }
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
    /// error implementing [`std::error::Error`].
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error().map(|error| error as &(dyn Error + 'static))
    }
}
