// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================
//! Tokio-gated asynchronous CAS attempt support.

use std::sync::{Arc, Mutex};

use qubit_atomic::AtomicRef;

use crate::cas_decision::CasDecision;
use crate::error::CasAttemptFailure;
use crate::executor::cas_executor::decision::apply_decision;
use crate::executor::internal::AttemptSuccess;

/// Runs one asynchronous attempt and records its snapshot for timeout errors.
pub(super) async fn run_async_attempt<T, R, E, O, Fut>(
    state: &AtomicRef<T>,
    operation: &O,
    attempt_snapshot: Arc<Mutex<Option<Arc<T>>>>,
) -> Result<AttemptSuccess<T, R>, CasAttemptFailure<T, E>>
where
    O: Fn(Arc<T>) -> Fut,
    Fut: std::future::Future<Output = CasDecision<T, R, E>>,
{
    let current = state.load();
    *attempt_snapshot
        .lock()
        .expect("CAS attempt snapshot slot should be lockable") = Some(Arc::clone(&current));
    let decision = operation(Arc::clone(&current)).await;
    apply_decision(state, current, decision)
}
