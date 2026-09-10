// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared projection of a CAS decision onto one retry attempt result.

use std::sync::Arc;

use qubit_atomic::AtomicRef;

use crate::cas_decision::CasDecision;
use crate::error::CasAttemptFailure;
use crate::executor::internal::AttemptSuccess;

/// Applies one CAS decision and preserves the atomic compare-and-set snapshot.
///
/// # Type Parameters
/// - `T`: Shared application state.
/// - `R`: Output moved to the caller only on success.
/// - `E`: Business failure retained on Retry or Abort.
///
/// # Parameters
/// - `state`: Atomic slot written only by an accepted Update.
/// - `current`: Snapshot used by the operation; Finish never revalidates it.
/// - `decision`: Owned operation decision and its output or business error.
///
/// # Returns
/// A committed update or a no-write finish, retaining that attempt's snapshots.
///
/// # Errors
/// Returns a conflict with the CAS-observed actual value, or the operation's
/// Retry/Abort failure with its original snapshot. Failed update output is
/// dropped.
pub(super) fn apply_decision<T, R, E>(
    state: &AtomicRef<T>,
    current: Arc<T>,
    decision: CasDecision<T, R, E>,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>> {
    match decision {
        CasDecision::Update { next, output } => match state.compare_set(&current, Arc::clone(&next)) {
            Ok(()) => Ok(AttemptSuccess::Updated {
                previous: current,
                current: next,
                output,
            }),
            Err(actual) => Err(CasAttemptFailure::conflict(actual)),
        },
        CasDecision::Finish { output } => Ok(AttemptSuccess::Finished { current, output }),
        CasDecision::Retry(error) => Err(CasAttemptFailure::retry(current, error)),
        CasDecision::Abort(error) => Err(CasAttemptFailure::abort(current, error)),
    }
}
