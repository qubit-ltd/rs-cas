// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================
//! Synchronous CAS attempt support.

use qubit_atomic::AtomicRef;
use qubit_function::Function;

use crate::cas_decision::CasDecision;
use crate::error::CasAttemptFailure;
use crate::executor::cas_executor::decision::apply_decision;
use crate::executor::internal::AttemptSuccess;

/// Runs one synchronous attempt after loading its state snapshot.
pub(super) fn run_sync_attempt<T, R, E, O>(
    state: &AtomicRef<T>,
    operation: &O,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
where
    O: Function<T, CasDecision<T, R, E>>,
{
    let current = state.load();
    let decision = operation.apply(current.as_ref());
    apply_decision(state, current, decision)
}
