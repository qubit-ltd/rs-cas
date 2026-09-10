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
use qubit_retry::RetryCallbackFailure;
use qubit_retry::RetryContext;
use qubit_retry::RetryError;
use qubit_retry::RetryErrorReason;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryTimeoutScope;

use super::CasAttemptFailure;
use super::CasDiagnostic;
use super::CasDiagnosticKind;
use super::CasErrorKind;
use super::CasLimitKind;
use super::CasTermination;
use super::CasTimeoutScope;
use super::internal::CasErrorDetails;
use super::internal::reason_diagnostic;
use crate::event::CasContext;

/// Terminal CAS error returned by [`crate::CasExecutor`].
///
/// # Type Parameters
/// - `T`: State snapshot retained from the last failed attempt.
/// - `E`: Original business failure, when one was retained.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasErrorKind;
///
/// let state = AtomicRef::from_value(3usize);
/// let error = CasExecutor::<usize, &'static str>::builder().build().unwrap()
///     .execute_result(&state, |_: &usize| CasDecision::<usize, (), _>::abort("sold out"))
///     .unwrap_err();
/// assert_eq!(error.kind(), CasErrorKind::Abort);
/// assert_eq!(error.error(), Some(&"sold out"));
/// assert_eq!(**error.current().unwrap(), 3);
/// assert_eq!(error.attempts(), 1);
/// ```
#[must_use = "a CAS error describes an unsuccessful execution"]
#[derive(Clone)]
pub struct CasError<T, E> {
    /// Terminal category, which takes precedence over the last business error.
    kind: CasErrorKind,
    /// Owned terminal context and infrastructure diagnostics, independent of
    /// T/E.
    details: Box<CasErrorDetails>,
    /// Final failed attempt and its snapshot; `None` means no retainable
    /// failure. A later budget or deadline may determine the terminal
    /// category instead.
    last_failure: Option<CasAttemptFailure<T, E>>,
}

impl<T, E> CasError<T, E> {
    /// Wraps one retry-layer error without exposing retry-layer types.
    ///
    /// # Parameters
    /// - `inner`: Owned terminal retry error and final attempt context.
    /// - `timeout_current`: `Some` original timed-out attempt snapshot, or
    ///   `None` when absent.
    ///
    /// # Returns
    /// A CAS-owned error preserving termination precedence and available
    /// failure data.
    #[inline]
    pub(crate) fn new(inner: RetryError<CasAttemptFailure<T, E>>, timeout_current: Option<Arc<T>>) -> Self {
        let (reason, last_failure, retry_context, diagnostics) = inner.into_parts();
        Self::from_retry_parts(reason, last_failure, retry_context, timeout_current, diagnostics)
    }

    /// Projects terminal metadata, the last attempt, and completion
    /// diagnostics.
    ///
    /// # Parameters
    /// - `reason`: Terminal retry reason, independent of the final business
    ///   error.
    /// - `last_failure`: `Some` final failed attempt, or `None` without one.
    /// - `retry_context`: Terminal counters and configured limits.
    /// - `timeout_current`: `Some` original snapshot for timeout projection, or
    ///   `None`.
    /// - `completion_failures`: Owned post-terminal callback failures in
    ///   dispatch order.
    ///
    /// # Returns
    /// An owned CAS error; no new state snapshot is loaded during projection.
    pub(crate) fn from_retry_parts(
        reason: RetryErrorReason,
        last_failure: Option<AttemptFailure<CasAttemptFailure<T, E>>>,
        retry_context: RetryContext,
        mut timeout_current: Option<Arc<T>>,
        completion_failures: Box<[RetryCallbackFailure]>,
    ) -> Self {
        let diagnostic = reason_diagnostic(&reason);
        let completion_diagnostics = completion_failures
            .into_vec()
            .into_iter()
            .map(|failure| CasDiagnostic::new(CasDiagnosticKind::Callback, failure.to_string()))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let context = CasContext::new(&retry_context);
        let retained = last_failure.and_then(|failure| Self::map_attempt_failure(failure, &mut timeout_current));
        let termination = Self::classify_termination(&reason);
        let kind = Self::classify_kind(termination, retained.as_ref());
        Self {
            kind,
            details: Box::new(CasErrorDetails {
                termination,
                context,
                diagnostic,
                completion_diagnostics,
            }),
            last_failure: retained,
        }
    }

    /// Returns infrastructure details, or `None` for ordinary CAS termination.
    ///
    /// # Returns
    /// `Some` retained infrastructure cause, or `None` for ordinary CAS
    /// termination.
    #[must_use]
    #[inline(always)]
    pub fn diagnostic(&self) -> Option<&CasDiagnostic> {
        self.details.diagnostic.as_ref()
    }

    /// Returns callback failures recorded after the terminal result was frozen.
    /// An empty slice means no completion callback failed.
    ///
    /// # Returns
    /// Ordered post-terminal callback diagnostics; an empty slice means none
    /// failed.
    #[must_use]
    #[inline(always)]
    pub fn completion_diagnostics(&self) -> &[CasDiagnostic] {
        &self.details.completion_diagnostics
    }

    /// Returns the high-level CAS error kind.
    ///
    /// # Returns
    /// The terminal category, accounting for budget and timeout precedence.
    #[must_use]
    #[inline(always)]
    pub fn kind(&self) -> CasErrorKind {
        self.kind
    }

    /// Returns the structured CAS terminal reason.
    ///
    /// # Returns
    /// The structured stopping reason independent of the retained attempt
    /// error.
    #[must_use]
    #[inline(always)]
    pub fn termination(&self) -> CasTermination {
        self.details.termination
    }

    /// Returns the terminal CAS context.
    ///
    /// # Returns
    /// A copied snapshot of retry counters and limits at termination.
    #[must_use]
    #[inline(always)]
    pub fn context(&self) -> CasContext {
        self.details.context
    }

    /// Returns the number of attempts that were executed.
    ///
    /// # Returns
    /// Started attempt count, possibly zero when stopped before admission.
    #[must_use]
    #[inline(always)]
    pub fn attempts(&self) -> u32 {
        self.details.context.attempts()
    }

    /// Returns `Some` final attempt failure, or `None` before an attempt or
    /// when infrastructure termination left no retainable application failure.
    ///
    /// # Returns
    /// `Some` borrowed attempt failure and its snapshot, or `None` if none was
    /// retained.
    #[must_use]
    #[inline(always)]
    pub fn last_failure(&self) -> Option<&CasAttemptFailure<T, E>> {
        self.last_failure.as_ref()
    }

    /// Returns `Some` retained failure snapshot without reloading shared state.
    /// Returns `None` when no application failure snapshot was retained.
    ///
    /// # Returns
    /// `Some` borrowed failure snapshot, or `None` without a retained snapshot.
    #[must_use]
    #[inline(always)]
    pub fn current(&self) -> Option<&Arc<T>> {
        self.last_failure().map(CasAttemptFailure::current)
    }

    /// Returns `Some` business error for Retry/Abort, or `None` for a conflict,
    /// timeout, or an execution without a retained application failure.
    ///
    /// # Returns
    /// `Some` original Retry/Abort business error, or `None` for other
    /// failures.
    #[must_use]
    #[inline(always)]
    pub fn error(&self) -> Option<&E> {
        self.last_failure().and_then(CasAttemptFailure::error)
    }

    /// Consumes this error and returns `Some` owned attempt failure, or `None`
    /// when no application failure was retained.
    ///
    /// # Returns
    /// `Some` owned attempt failure, or `None` when no retainable failure
    /// exists.
    #[must_use]
    #[inline(always)]
    pub fn into_last_failure(self) -> Option<CasAttemptFailure<T, E>> {
        self.last_failure
    }

    /// Retains application failures or the original snapshot of a timed-out
    /// attempt.
    ///
    /// # Parameters
    /// - `failure`: Owned retry-layer attempt failure.
    /// - `timeout_current`: Snapshot consumed only when the failure is a
    ///   timeout.
    ///
    /// # Returns
    /// `Some` original business/CAS failure or timeout with a retained
    /// snapshot; `None` for a panic, unknown failure, or timeout without a
    /// snapshot.
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

    /// Classifies the terminal reason independently of the last application
    /// error.
    ///
    /// # Parameters
    /// - `reason`: Terminal reason borrowed from the retry facade.
    ///
    /// # Returns
    /// A CAS stopping reason; unknown upstream reasons remain infrastructure
    /// failures.
    #[must_use]
    fn classify_termination(reason: &RetryErrorReason) -> CasTermination {
        match reason {
            RetryErrorReason::Aborted => CasTermination::Aborted,
            RetryErrorReason::Exhausted { limit } => CasTermination::LimitExceeded(match limit {
                RetryLimitKind::Attempts => CasLimitKind::Attempts,
                RetryLimitKind::OperationElapsed => CasLimitKind::OperationElapsed,
                RetryLimitKind::TotalElapsed => CasLimitKind::TotalElapsed,
            }),
            RetryErrorReason::TimedOut { scope } => CasTermination::TimedOut(match scope {
                RetryTimeoutScope::Attempt => CasTimeoutScope::Attempt,
                RetryTimeoutScope::Flow => CasTimeoutScope::Flow,
            }),
            RetryErrorReason::Cancelled { .. }
            | RetryErrorReason::CallbackFailed { .. }
            | RetryErrorReason::Infrastructure { .. } => CasTermination::RetryInfrastructure,
            _ => CasTermination::RetryInfrastructure,
        }
    }

    /// Projects terminal precedence onto the compact public error category.
    ///
    /// # Parameters
    /// - `termination`: Structured reason that stopped the flow.
    /// - `last_failure`: `Some` last attempt to refine exhaustion/abort, or
    ///   `None`.
    ///
    /// # Returns
    /// A compact category preserving budget and flow-timeout precedence.
    #[must_use]
    fn classify_kind(termination: CasTermination, last_failure: Option<&CasAttemptFailure<T, E>>) -> CasErrorKind {
        match termination {
            CasTermination::Aborted => match last_failure {
                Some(CasAttemptFailure::Timeout { .. }) => CasErrorKind::AttemptTimeout,
                _ => CasErrorKind::Abort,
            },
            CasTermination::LimitExceeded(CasLimitKind::Attempts) => match last_failure {
                Some(CasAttemptFailure::Conflict { .. }) => CasErrorKind::ConflictExhausted,
                Some(CasAttemptFailure::Timeout { .. }) => CasErrorKind::AttemptTimeout,
                _ => CasErrorKind::RetryExhausted,
            },
            CasTermination::LimitExceeded(CasLimitKind::OperationElapsed) => CasErrorKind::OperationBudgetExceeded,
            CasTermination::LimitExceeded(CasLimitKind::TotalElapsed) => CasErrorKind::TotalBudgetExceeded,
            CasTermination::TimedOut(CasTimeoutScope::Attempt) => CasErrorKind::AttemptTimeout,
            CasTermination::TimedOut(CasTimeoutScope::Flow) => CasErrorKind::FlowTimeout,
            CasTermination::RetryInfrastructure => CasErrorKind::RetryInfrastructure,
        }
    }
}

impl<T, E> fmt::Debug for CasError<T, E> {
    /// Formats the terminal classification and retained diagnostic details.
    ///
    /// # Parameters
    /// - `f`: Destination formatter.
    ///
    /// # Returns
    /// Formatting completion status.
    ///
    /// # Errors
    /// Returns a formatting error if the destination rejects a write.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CasError")
            .field("kind", &self.kind)
            .field("termination", &self.termination())
            .field("context", &self.context())
            .field("diagnostic", &self.diagnostic())
            .field("completion_diagnostics", &self.completion_diagnostics())
            .finish()
    }
}

impl<T, E> fmt::Display for CasError<T, E>
where
    E: fmt::Display,
{
    /// Formats the terminal classification and retained diagnostic details.
    ///
    /// # Parameters
    /// - `f`: Destination formatter.
    ///
    /// # Returns
    /// Formatting completion status.
    ///
    /// # Errors
    /// Returns a formatting error if the destination rejects a write.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind() {
            CasErrorKind::Abort => "CAS aborted",
            CasErrorKind::ConflictExhausted => "CAS conflicts exhausted",
            CasErrorKind::RetryExhausted => "CAS retryable failures exhausted",
            CasErrorKind::AttemptTimeout => "CAS attempt timed out",
            CasErrorKind::FlowTimeout => "CAS flow timed out",
            CasErrorKind::RetryInfrastructure => "CAS retry infrastructure failed",
            CasErrorKind::OperationBudgetExceeded => "CAS operation budget exceeded",
            CasErrorKind::TotalBudgetExceeded => "CAS total budget exceeded",
        };
        write!(f, "{message} after {} attempt(s)", self.attempts())?;
        if let Some(failure) = self.last_failure() {
            write!(f, "; last failure: {failure}")?;
        }
        if let Some(diagnostic) = self.diagnostic() {
            write!(f, "; diagnostic: {diagnostic}")?;
        }
        for diagnostic in self.completion_diagnostics() {
            write!(f, "; completion diagnostic: {diagnostic}")?;
        }
        Ok(())
    }
}

impl<T, E> Error for CasError<T, E>
where
    E: Error + 'static,
{
    /// Returns the original business error when one was retained.
    ///
    /// # Returns
    /// `Some` borrowed original business error, or `None` when termination
    /// retained none.
    #[inline(always)]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error().map(|error| error as &(dyn Error + 'static))
    }
}
