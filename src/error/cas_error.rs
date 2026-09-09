// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
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
#[derive(Clone)]
pub struct CasError<T, E> {
    kind: CasErrorKind,
    details: Box<CasErrorDetails>,
    last_failure: Option<CasAttemptFailure<T, E>>,
}

impl<T, E> CasError<T, E> {
    /// Wraps one retry-layer error without exposing retry-layer types.
    pub(crate) fn new(inner: RetryError<CasAttemptFailure<T, E>>, timeout_current: Option<Arc<T>>) -> Self {
        let (reason, last_failure, retry_context, diagnostics) = inner.into_parts();
        Self::from_retry_parts(reason, last_failure, retry_context, timeout_current, diagnostics)
    }

    /// Projects terminal metadata, the last attempt, and completion
    /// diagnostics.
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

    /// Retains application failures or the original snapshot of a timed-out
    /// attempt.
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

    /// Returns infrastructure details, or `None` for ordinary CAS termination.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&CasDiagnostic> {
        self.details.diagnostic.as_ref()
    }

    /// Returns callback failures recorded after the terminal result was frozen.
    /// An empty slice means no completion callback failed.
    #[must_use]
    pub fn completion_diagnostics(&self) -> &[CasDiagnostic] {
        &self.details.completion_diagnostics
    }

    /// Returns the high-level CAS error kind.
    #[must_use]
    pub fn kind(&self) -> CasErrorKind {
        self.kind
    }

    /// Returns the structured CAS terminal reason.
    #[must_use]
    pub fn termination(&self) -> CasTermination {
        self.details.termination
    }

    /// Returns the terminal CAS context.
    #[must_use]
    pub fn context(&self) -> CasContext {
        self.details.context
    }

    /// Returns the number of attempts that were executed.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.details.context.attempts()
    }

    /// Returns the retained application-level CAS failure, when one exists.
    #[must_use]
    pub fn last_failure(&self) -> Option<&CasAttemptFailure<T, E>> {
        self.last_failure.as_ref()
    }

    /// Consumes this error and returns its retained application failure.
    #[must_use]
    pub fn into_last_failure(self) -> Option<CasAttemptFailure<T, E>> {
        self.last_failure
    }

    /// Returns the current state associated with the last failure.
    #[must_use]
    pub fn current(&self) -> Option<&Arc<T>> {
        self.last_failure().map(CasAttemptFailure::current)
    }

    /// Returns the business error associated with the last failure.
    #[must_use]
    pub fn error(&self) -> Option<&E> {
        self.last_failure().and_then(CasAttemptFailure::error)
    }
}

impl<T, E> fmt::Debug for CasError<T, E> {
    /// Formats the terminal classification and retained diagnostic details.
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
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error().map(|error| error as &(dyn Error + 'static))
    }
}
