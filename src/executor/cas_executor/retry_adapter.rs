// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================
//! Shared retry classification for rich and result-only execution paths.

use qubit_retry::{AttemptFailure, RetryDecision, RetryTimeoutScope};

use crate::error::CasAttemptFailure;
use crate::executor::internal::AttemptTimeoutAction;

/// Classifies one retry-layer failure without producing side effects.
pub(super) fn retry_decision<T, E>(
    failure: &AttemptFailure<CasAttemptFailure<T, E>>,
    attempt_timeout_action: AttemptTimeoutAction,
) -> RetryDecision {
    match failure {
        AttemptFailure::Error(CasAttemptFailure::Conflict { .. })
        | AttemptFailure::Error(CasAttemptFailure::Retry { .. }) => RetryDecision::Retry,
        AttemptFailure::Error(CasAttemptFailure::Abort { .. }) => RetryDecision::Abort,
        AttemptFailure::Error(CasAttemptFailure::Timeout { .. }) => RetryDecision::UseDefault,
        AttemptFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt,
        } if attempt_timeout_action == AttemptTimeoutAction::Retry => RetryDecision::Retry,
        AttemptFailure::TimedOut { .. } | AttemptFailure::Panicked { .. } => {
            RetryDecision::UseDefault
        }
        _ => RetryDecision::UseDefault,
    }
}
