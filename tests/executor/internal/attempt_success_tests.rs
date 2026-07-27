// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

use crate::support::TestError;

/// Verifies result-only execution enriches an updated attempt success.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_result_only_execution_enriches_updated_success() {
    let state = AtomicRef::from_value(4usize);
    let executor = CasExecutor::<usize, TestError>::latency_first();

    let success = executor
        .execute_result(&state, |current: &usize| {
            CasDecision::update(*current + 1, *current)
        })
        .expect("result-only update should succeed");

    assert!(success.is_updated());
    assert_eq!(**success.previous().expect("update has previous state"), 4);
    assert_eq!(**success.current(), 5);
    assert_eq!(*success.output(), 4);
    assert_eq!(success.attempts(), 1);
}
