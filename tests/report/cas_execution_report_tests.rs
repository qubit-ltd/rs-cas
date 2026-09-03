// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutionReport;
use qubit_cas::CasExecutor;
use qubit_cas::ContentionThresholds;
use qubit_cas::constants::DEFAULT_CAS_MAX_ATTEMPTS;

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

/// Verifies direct report accessors handle an execution without conflicts.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_report_accessors_cover_terminal_success() {
    let state = AtomicRef::from_value(1usize);
    let outcome = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .max_operation_elapsed(Some(Duration::from_secs(1)))
        .max_total_elapsed(Some(Duration::from_secs(2)))
        .no_delay()
        .build()
        .expect("executor should build")
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::finish(())
        });
    let report = std::hint::black_box(outcome.report().clone());

    assert_eq!(std::hint::black_box(report.attempts_total()), 1);
    assert_eq!(std::hint::black_box(report.conflicts()), 0);
    assert_eq!(std::hint::black_box(report.retry_errors()), 0);
    assert_eq!(std::hint::black_box(report.aborts()), 0);
    assert_eq!(std::hint::black_box(report.timeouts()), 0);
    assert!(std::hint::black_box(report.finished_at()) >= report.started_at());
    assert!(std::hint::black_box(report.elapsed()) >= Duration::ZERO);
    assert_eq!(std::hint::black_box(report.max_attempts()), 2);
    assert_eq!(
        std::hint::black_box(report.max_operation_elapsed()),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        std::hint::black_box(report.max_total_elapsed()),
        Some(Duration::from_secs(2))
    );
    assert_eq!(std::hint::black_box(report.retryable_failure_ratio()), 0.0);
}

/// Verifies every report accessor can be used through its public function type.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_report_accessor_function_pointers_work() {
    let state = AtomicRef::from_value(1usize);
    let report = CasExecutor::<usize, TestError>::builder()
        .build()
        .expect("executor should build")
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::finish(())
        })
        .report()
        .clone();

    let attempts_total: fn(&CasExecutionReport) -> u32 = CasExecutionReport::attempts_total;
    let conflicts: fn(&CasExecutionReport) -> u32 = CasExecutionReport::conflicts;
    let retry_errors: fn(&CasExecutionReport) -> u32 = CasExecutionReport::retry_errors;
    let aborts: fn(&CasExecutionReport) -> u32 = CasExecutionReport::aborts;
    let timeouts: fn(&CasExecutionReport) -> u32 = CasExecutionReport::timeouts;
    let started_at: fn(&CasExecutionReport) -> std::time::Instant = CasExecutionReport::started_at;
    let finished_at: fn(&CasExecutionReport) -> std::time::Instant = CasExecutionReport::finished_at;
    let elapsed: fn(&CasExecutionReport) -> Duration = CasExecutionReport::elapsed;
    let max_attempts: fn(&CasExecutionReport) -> u32 = CasExecutionReport::max_attempts;
    let max_operation_elapsed: fn(&CasExecutionReport) -> Option<Duration> = CasExecutionReport::max_operation_elapsed;
    let max_total_elapsed: fn(&CasExecutionReport) -> Option<Duration> = CasExecutionReport::max_total_elapsed;
    let outcome: fn(&CasExecutionReport) -> CasExecutionOutcome = CasExecutionReport::outcome;
    let conflict_ratio: fn(&CasExecutionReport) -> f64 = CasExecutionReport::conflict_ratio;
    let retryable_failure_ratio: fn(&CasExecutionReport) -> f64 = CasExecutionReport::retryable_failure_ratio;
    let is_contention_hot: fn(&CasExecutionReport, &ContentionThresholds) -> bool =
        CasExecutionReport::is_contention_hot;

    assert_eq!(attempts_total(&report), 1);
    assert_eq!(conflicts(&report), 0);
    assert_eq!(retry_errors(&report), 0);
    assert_eq!(aborts(&report), 0);
    assert_eq!(timeouts(&report), 0);
    assert!(finished_at(&report) >= started_at(&report));
    assert!(elapsed(&report) >= Duration::ZERO);
    assert_eq!(max_attempts(&report), DEFAULT_CAS_MAX_ATTEMPTS);
    assert_eq!(max_operation_elapsed(&report), None);
    assert_eq!(max_total_elapsed(&report), None);
    assert_eq!(outcome(&report), CasExecutionOutcome::SuccessFinished);
    assert_eq!(conflict_ratio(&report), 0.0);
    assert_eq!(retryable_failure_ratio(&report), 0.0);
    assert!(is_contention_hot(&report, &ContentionThresholds::new(1, 0, 0.0)));
}
