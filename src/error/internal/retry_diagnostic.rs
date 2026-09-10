// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Projection of retry infrastructure reasons into CAS-owned diagnostics.

use qubit_retry::RetryErrorReason;
use qubit_retry::RetryInfrastructureFailure;

use crate::error::CasDiagnostic;
use crate::error::CasDiagnosticKind;

/// Retains details for infrastructure reasons; ordinary CAS termination has
/// none.
///
/// # Parameters
/// - `reason`: Retry stopping reason to inspect.
///
/// # Returns
/// `Some` owned infrastructure classification and text, or `None` for ordinary
/// abort, exhaustion, and timeout termination.
pub(in crate::error) fn reason_diagnostic(reason: &RetryErrorReason) -> Option<CasDiagnostic> {
    let kind = match reason {
        RetryErrorReason::Aborted | RetryErrorReason::Exhausted { .. } | RetryErrorReason::TimedOut { .. } => {
            return None;
        }
        RetryErrorReason::Cancelled { .. } => CasDiagnosticKind::Cancellation,
        RetryErrorReason::CallbackFailed { .. } => CasDiagnosticKind::Callback,
        RetryErrorReason::Infrastructure { failure } => {
            if matches!(failure, RetryInfrastructureFailure::Clock { .. }) {
                CasDiagnosticKind::Clock
            } else if matches!(failure, RetryInfrastructureFailure::Timer { .. }) {
                CasDiagnosticKind::Timer
            } else {
                CasDiagnosticKind::Infrastructure
            }
        }
        _ => CasDiagnosticKind::UnknownRetryReason,
    };
    Some(CasDiagnostic::new(kind, reason.to_string()))
}
