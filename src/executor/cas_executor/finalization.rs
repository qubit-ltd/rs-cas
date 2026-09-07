// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Finalization of retry executions into public CAS outcomes.

use std::sync::Arc;
use std::sync::Mutex;

use qubit_retry::RetryContext;
use qubit_retry::RetryError;
use qubit_retry::RetrySuccess;

use super::CasExecutor;
use crate::cas_outcome::CasOutcome;
use crate::cas_success::CasSuccess;
use crate::error::CasAttemptFailure;
use crate::error::CasError;
use crate::error::CasErrorKind;
use crate::event::CasContext;
use crate::event::CasHooks;
use crate::executor::internal::AttemptSuccess;
use crate::executor::internal::CasReportFinishContext;
use crate::report::CasExecutionOutcome;
use crate::report::CasReportBuilder;

/// Enriches an attempt success with retry context.
pub(super) fn enrich_success<T, R>(success: AttemptSuccess<T, R>, context: RetryContext) -> CasSuccess<T, R> {
    let context = CasContext::new(&context);
    match success {
        AttemptSuccess::Updated {
            previous,
            current,
            output,
        } => CasSuccess::updated(previous, current, output, context),
        AttemptSuccess::Finished { current, output } => CasSuccess::finished(current, output, context),
    }
}

/// Maps a terminal CAS error kind to its report outcome.
pub(super) fn error_outcome(kind: CasErrorKind) -> CasExecutionOutcome {
    match kind {
        CasErrorKind::Abort => CasExecutionOutcome::ErrorAbort,
        CasErrorKind::Conflict => CasExecutionOutcome::ErrorConflictExhausted,
        CasErrorKind::RetryExhausted => CasExecutionOutcome::ErrorRetryExhausted,
        CasErrorKind::AttemptTimeout => CasExecutionOutcome::ErrorAttemptTimeout,
        CasErrorKind::RetryInfrastructure => CasExecutionOutcome::ErrorRetryInfrastructure,
        CasErrorKind::MaxOperationElapsedExceeded => CasExecutionOutcome::ErrorMaxOperationElapsedExceeded,
        CasErrorKind::MaxTotalElapsedExceeded => CasExecutionOutcome::ErrorMaxTotalElapsedExceeded,
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Finalizes one retry execution into the public CAS result type.
    ///
    /// # Parameters
    /// - `attempt`: Retry-layer terminal success or error.
    /// - `hooks`: Hook registrations for the current execution.
    /// - `attempt_snapshot`: Last async operation snapshot, when an async
    ///   execution needs to preserve it for a timeout error.
    ///
    /// # Returns
    /// Public CAS success or error.
    pub(super) fn finish_execution<R>(
        &self,
        attempt: Result<RetrySuccess<AttemptSuccess<T, R>>, RetryError<CasAttemptFailure<T, E>>>,
        hooks: CasHooks,
        attempt_snapshot: Option<Arc<Mutex<Option<Arc<T>>>>>,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> CasOutcome<T, R, E>
    where
        T: 'static,
        E: 'static,
    {
        match attempt {
            Ok(success) => {
                // This adapter registers no completion observers; only retry context is
                // projected.
                let (success, context, _diagnostics) = success.into_parts();
                let attempts_total = context.attempts();
                let max_attempts = context.max_attempts();
                let max_operation_elapsed = context.max_operation_elapsed();
                let max_total_elapsed = context.max_total_elapsed();
                let outcome = match success {
                    AttemptSuccess::Updated { .. } => CasExecutionOutcome::SuccessUpdated,
                    AttemptSuccess::Finished { .. } => CasExecutionOutcome::SuccessFinished,
                };
                let success = super::finalization::enrich_success(success, context);
                let report = self.finish_report(
                    &hooks,
                    report_builder,
                    CasReportFinishContext::new(
                        attempts_total,
                        max_attempts,
                        max_operation_elapsed,
                        max_total_elapsed,
                        outcome,
                    ),
                );
                CasOutcome::new(Ok(success), report)
            }
            Err(error) => {
                let timeout_current = attempt_snapshot.and_then(|snapshot| {
                    snapshot
                        .lock()
                        .expect("CAS attempt snapshot slot should be lockable")
                        .clone()
                });
                let error = CasError::new(error, timeout_current);
                let context = error.context();
                let outcome = super::finalization::error_outcome(error.kind());
                let report = self.finish_report(
                    &hooks,
                    report_builder,
                    CasReportFinishContext::new(
                        context.attempts(),
                        context.max_attempts(),
                        context.max_operation_elapsed(),
                        context.max_total_elapsed(),
                        outcome,
                    ),
                );
                CasOutcome::new(Err(error), report)
            }
        }
    }
}
