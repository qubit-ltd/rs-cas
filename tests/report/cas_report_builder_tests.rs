// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutionOutcome, CasExecutor};

use crate::support::TestError;

#[test]
fn test_report_builder_records_conflicts_and_retry_errors_via_executor() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| {
        match attempts.fetch_add(1, Ordering::SeqCst) {
            0 => {
                state.store(Arc::new(*current + 1));
                CasDecision::update(*current + 1, ())
            }
            1 => CasDecision::retry(TestError("retry-once")),
            _ => CasDecision::finish(()),
        }
    });
    let report = outcome.report();

    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessFinished);
    assert_eq!(report.attempts_total(), 3);
    assert_eq!(report.conflicts(), 1);
    assert_eq!(report.retry_errors(), 1);
    assert_eq!(report.aborts(), 0);
    assert_eq!(report.timeouts(), 0);
}
