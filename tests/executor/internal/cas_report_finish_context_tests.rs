// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutionOutcome, CasExecutor};

use crate::support::TestError;

/// Verifies report finalization preserves retry limits and terminal outcome.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_report_finalization_preserves_limits_and_outcome() {
    let state = AtomicRef::from_value(8usize);
    let max_operation_elapsed = Duration::from_secs(2);
    let max_total_elapsed = Duration::from_secs(3);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(4)
        .max_operation_elapsed(Some(max_operation_elapsed))
        .max_total_elapsed(Some(max_total_elapsed))
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |_current: &usize| {
        CasDecision::<usize, (), TestError>::finish(())
    });

    assert_eq!(outcome.report().attempts_total(), 1);
    assert_eq!(outcome.report().max_attempts(), 4);
    assert_eq!(
        outcome.report().max_operation_elapsed(),
        Some(max_operation_elapsed)
    );
    assert_eq!(
        outcome.report().max_total_elapsed(),
        Some(max_total_elapsed)
    );
    assert_eq!(
        outcome.report().outcome(),
        CasExecutionOutcome::SuccessFinished
    );
}
