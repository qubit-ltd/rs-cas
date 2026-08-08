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
use qubit_retry::RetryError;
use qubit_retry::RetryErrorReason;

use super::CasAttemptFailure;
use super::CasErrorKind;
use crate::event::CasContext;

/// Terminal CAS error returned by [`crate::CasExecutor`].
#[derive(Clone)]
pub struct CasError<T, E> {
    /// Cached high-level CAS error kind.
    kind: CasErrorKind,
    /// Terminal reason selected by the retry layer.
    reason: RetryErrorReason,
    /// Copied CAS context captured when execution stopped.
    context: CasContext,
    /// Last attempt-level CAS failure, when one exists.
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
    pub(crate) fn new(
        inner: RetryError<CasAttemptFailure<T, E>>,
        timeout_current: Option<Arc<T>>,
    ) -> Self {
        let (reason, raw_last_failure, retry_context) = inner.into_parts();
        let context = CasContext::new(&retry_context);
        let last_failure = match raw_last_failure {
            Some(AttemptFailure::Error(failure)) => Some(failure),
            Some(AttemptFailure::Timeout) => {
                timeout_current.map(CasAttemptFailure::timeout)
            }
            Some(AttemptFailure::Panic(_))
            | Some(AttemptFailure::Executor(_))
            | None => None,
        };
        let kind = Self::classify_kind(reason, last_failure.as_ref());
        Self {
            kind,
            reason,
            context,
            last_failure,
        }
    }

    /// Returns the classified CAS error kind.
    ///
    /// # Returns
    /// High-level CAS error kind derived from the retry-layer reason and last
    /// attempt failure.
    #[inline(always)]
    pub fn kind(&self) -> CasErrorKind {
        self.kind
    }

    /// Returns the retry-layer terminal reason.
    ///
    /// # Returns
    /// Underlying [`RetryErrorReason`].
    #[inline(always)]
    pub fn reason(&self) -> RetryErrorReason {
        self.reason
    }

    /// Returns the terminal CAS context.
    ///
    /// # Returns
    /// Copied CAS context captured when execution stopped.
    #[inline(always)]
    pub fn context(&self) -> CasContext {
        self.context
    }

    /// Returns the number of attempts that were executed.
    ///
    /// # Returns
    /// One-based attempt count.
    #[inline(always)]
    pub fn attempts(&self) -> u32 {
        self.context.attempt()
    }

    /// Returns the last CAS attempt failure when one exists.
    ///
    /// # Returns
    /// `Some(&CasAttemptFailure<T, E>)` when at least one attempt failed.
    #[inline(always)]
    pub fn last_failure(&self) -> Option<&CasAttemptFailure<T, E>> {
        self.last_failure.as_ref()
    }

    /// Returns the current state associated with the last failure.
    ///
    /// # Returns
    /// `Some(&Arc<T>)` when the terminal error preserved a current state.
    #[inline(always)]
    pub fn current(&self) -> Option<&Arc<T>> {
        self.last_failure().map(CasAttemptFailure::current)
    }

    /// Returns the business error associated with the last failure.
    ///
    /// # Returns
    /// `Some(&E)` for retryable or aborting business failures.
    #[inline(always)]
    pub fn error(&self) -> Option<&E> {
        self.last_failure().and_then(CasAttemptFailure::error)
    }

    /// Classifies one terminal CAS error kind from retry reason and failure.
    ///
    /// # Parameters
    /// - `reason`: Terminal reason selected by the retry layer.
    /// - `last_failure`: Last CAS failure when one exists.
    ///
    /// # Returns
    /// Derived high-level CAS error kind.
    fn classify_kind(
        reason: RetryErrorReason,
        last_failure: Option<&CasAttemptFailure<T, E>>,
    ) -> CasErrorKind {
        match reason {
            RetryErrorReason::Aborted => match last_failure {
                Some(CasAttemptFailure::Timeout { .. }) => {
                    CasErrorKind::AttemptTimeout
                }
                _ => CasErrorKind::Abort,
            },
            RetryErrorReason::AttemptsExceeded => match last_failure {
                Some(CasAttemptFailure::Conflict { .. }) => {
                    CasErrorKind::Conflict
                }
                Some(CasAttemptFailure::Timeout { .. }) => {
                    CasErrorKind::AttemptTimeout
                }
                _ => CasErrorKind::RetryExhausted,
            },
            RetryErrorReason::UnsupportedOperation => {
                CasErrorKind::UnsupportedOperation
            }
            RetryErrorReason::SleeperFailed
            | RetryErrorReason::WorkerStillRunning => {
                CasErrorKind::RetryInfrastructure
            }
            RetryErrorReason::MaxOperationElapsedExceeded => {
                CasErrorKind::MaxOperationElapsedExceeded
            }
            RetryErrorReason::MaxTotalElapsedExceeded => {
                CasErrorKind::MaxTotalElapsedExceeded
            }
        }
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
            .field("reason", &self.reason())
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
            CasErrorKind::UnsupportedOperation => {
                "CAS unsupported operation for this execution mode"
            }
            CasErrorKind::RetryInfrastructure => {
                "CAS retry infrastructure failed"
            }
            CasErrorKind::MaxOperationElapsedExceeded => {
                "CAS max operation elapsed exceeded"
            }
            CasErrorKind::MaxTotalElapsedExceeded => {
                "CAS max total elapsed exceeded"
            }
        };
        write!(f, "{message} after {} attempt(s)", self.attempts())?;
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
