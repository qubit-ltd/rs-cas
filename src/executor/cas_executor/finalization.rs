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
///
/// # Type Parameters
/// - `T`: Shared state held by the successful attempt.
/// - `R`: Business output moved from that attempt.
///
/// # Parameters
/// - `success`: Committed update or no-write result.
/// - `context`: Terminal retry counters and limits.
///
/// # Returns
/// A public success retaining the original attempt snapshots and output.
#[must_use]
#[inline]
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
///
/// # Parameters
/// - `kind`: Terminal error category after precedence has been resolved.
///
/// # Returns
/// The corresponding report outcome, without reclassifying the last attempt.
#[must_use]
pub(super) fn error_outcome(kind: CasErrorKind) -> CasExecutionOutcome {
    match kind {
        CasErrorKind::Abort => CasExecutionOutcome::ErrorAbort,
        CasErrorKind::ConflictExhausted => CasExecutionOutcome::ErrorConflictExhausted,
        CasErrorKind::RetryExhausted => CasExecutionOutcome::ErrorRetryExhausted,
        CasErrorKind::AttemptTimeout => CasExecutionOutcome::ErrorAttemptTimeout,
        CasErrorKind::FlowTimeout => CasExecutionOutcome::ErrorFlowTimeout,
        CasErrorKind::RetryInfrastructure => CasExecutionOutcome::ErrorRetryInfrastructure,
        CasErrorKind::OperationBudgetExceeded => CasExecutionOutcome::ErrorOperationBudgetExceeded,
        CasErrorKind::TotalBudgetExceeded => CasExecutionOutcome::ErrorTotalBudgetExceeded,
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Finalizes one retry execution into the public CAS result type.
    ///
    /// # Type Parameters
    /// - `R`: Business output retained only from the successful attempt.
    ///
    /// # Parameters
    /// - `attempt`: Retry-layer terminal success or error.
    /// - `hooks`: Hook registrations for the current execution.
    /// - `timeout_current`: `Some` last async operation snapshot retained for
    ///   timeout projection; `None` for synchronous execution or no snapshot.
    /// - `report_builder`: Accumulated counters and isolated listener failures.
    ///
    /// # Returns
    /// Public CAS success or error.
    ///
    /// # Panics
    /// Panics if an internal report mutex has been poisoned.
    pub(super) fn finish_execution<R>(
        &self,
        attempt: Result<RetrySuccess<AttemptSuccess<T, R>>, RetryError<CasAttemptFailure<T, E>>>,
        hooks: CasHooks,
        timeout_current: Option<Arc<T>>,
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
                let (success, context, diagnostics) = success.into_parts();
                debug_assert!(diagnostics.is_empty(), "CAS installs no completion callbacks");
                let attempts_total = context.attempts();
                let max_attempts = context.max_attempts();
                let max_operation_elapsed = context.operation_time_budget();
                let max_total_elapsed = context.total_time_budget();
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
