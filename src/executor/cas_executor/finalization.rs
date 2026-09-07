// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================
//! Finalization boundary for CAS execution.
//!
//! Report ownership and public outcome construction stay on `CasExecutor`,
//! while pure projections live here so sync and async paths share them.

use qubit_retry::RetryContext;

use crate::cas_success::CasSuccess;
use crate::error::CasErrorKind;
use crate::event::CasContext;
use crate::executor::internal::AttemptSuccess;
use crate::report::CasExecutionOutcome;

/// Enriches one attempt success with retry context.
pub(super) fn enrich_success<T, R>(
    success: AttemptSuccess<T, R>,
    context: RetryContext,
) -> CasSuccess<T, R> {
    let context = CasContext::new(&context);
    match success {
        AttemptSuccess::Updated {
            previous,
            current,
            output,
        } => CasSuccess::updated(previous, current, output, context),
        AttemptSuccess::Finished { current, output } => {
            CasSuccess::finished(current, output, context)
        }
    }
}

/// Converts a terminal error kind into a report outcome.
pub(super) fn error_outcome(kind: CasErrorKind) -> CasExecutionOutcome {
    match kind {
        CasErrorKind::Abort => CasExecutionOutcome::ErrorAbort,
        CasErrorKind::Conflict => CasExecutionOutcome::ErrorConflictExhausted,
        CasErrorKind::RetryExhausted => CasExecutionOutcome::ErrorRetryExhausted,
        CasErrorKind::AttemptTimeout => CasExecutionOutcome::ErrorAttemptTimeout,
        CasErrorKind::RetryInfrastructure => CasExecutionOutcome::ErrorRetryInfrastructure,
        CasErrorKind::MaxOperationElapsedExceeded => {
            CasExecutionOutcome::ErrorMaxOperationElapsedExceeded
        }
        CasErrorKind::MaxTotalElapsedExceeded => CasExecutionOutcome::ErrorMaxTotalElapsedExceeded,
    }
}
