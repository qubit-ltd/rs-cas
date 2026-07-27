// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutionOutcome, CasExecutor, ContentionThresholds};

use crate::support::TestError;

/// Verifies report fields and ratios are populated after a conflict retry.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_report_exposes_counts_ratios_and_limits() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let max_operation_elapsed = Duration::from_secs(2);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .max_operation_elapsed(Some(max_operation_elapsed))
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| {
        if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            state.store(Arc::new(*current + 1));
        }
        CasDecision::update(*current + 1, *current + 10)
    });
    let report = outcome.report().clone();
    let _success = outcome.expect("second attempt should succeed");

    assert_eq!(report.attempts_total(), 2);
    assert_eq!(report.conflicts(), 1);
    assert_eq!(report.retry_errors(), 0);
    assert_eq!(report.aborts(), 0);
    assert_eq!(report.timeouts(), 0);
    assert_eq!(report.max_attempts(), 3);
    assert_eq!(report.max_operation_elapsed(), Some(max_operation_elapsed));
    assert_eq!(report.max_total_elapsed(), None);
    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
    assert_eq!(report.conflict_ratio(), 0.5);
    assert_eq!(report.retryable_failure_ratio(), 0.0);
    assert!(report.finished_at() >= report.started_at());
    assert!(report.elapsed() <= report.finished_at().duration_since(report.started_at()));
}

/// Verifies contention thresholds are evaluated from report statistics.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_report_contention_threshold_matching() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| {
        if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            state.store(Arc::new(*current + 1));
        }
        CasDecision::update(*current + 1, ())
    });
    let report = outcome.report().clone();
    let _success = outcome.expect("second attempt should succeed");

    let hot = ContentionThresholds::new(2, 1, 0.5);
    let cold = ContentionThresholds::new(3, 1, 0.5);

    assert!(report.is_contention_hot(&hot));
    assert!(!report.is_contention_hot(&cold));
}
