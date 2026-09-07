// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================
//! Shared projection of a CAS decision onto one retry attempt result.

use std::sync::Arc;

use qubit_atomic::AtomicRef;

use crate::cas_decision::CasDecision;
use crate::error::CasAttemptFailure;
use crate::executor::internal::AttemptSuccess;

/// Applies one CAS decision and preserves the atomic compare-and-set snapshot.
pub(super) fn apply_decision<T, R, E>(
    state: &AtomicRef<T>,
    current: Arc<T>,
    decision: CasDecision<T, R, E>,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>> {
    match decision {
        CasDecision::Update { next, output } => {
            match state.compare_set(&current, Arc::clone(&next)) {
                Ok(()) => Ok(AttemptSuccess::Updated {
                    previous: current,
                    current: next,
                    output,
                }),
                Err(actual) => Err(CasAttemptFailure::conflict(actual)),
            }
        }
        CasDecision::Finish { output } => Ok(AttemptSuccess::Finished { current, output }),
        CasDecision::Retry(error) => Err(CasAttemptFailure::retry(current, error)),
        CasDecision::Abort(error) => Err(CasAttemptFailure::abort(current, error)),
    }
}
