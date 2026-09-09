// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Equivalent decisions and soft budgets across synchronous facades.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasError;
use qubit_cas::CasErrorKind;
use qubit_cas::CasEvent;
use qubit_cas::CasExecutionReport;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::CasLimitKind;
use qubit_cas::CasSuccess;
use qubit_cas::CasTermination;

type ExecutionResult = Result<CasSuccess<usize, usize>, CasError<usize, &'static str>>;

/// Runs one script through a selected public facade, keeping optional
/// diagnostics.
fn run_path<F>(
    path: u8,
    executor: &CasExecutor<usize, &'static str>,
    state: &AtomicRef<usize>,
    operation: F,
) -> (ExecutionResult, Option<CasExecutionReport>)
where
    F: Fn(&usize) -> CasDecision<usize, usize, &'static str>,
{
    match path {
        0 => (executor.execute_result(state, operation), None),
        1 => {
            let (result, report) = executor.execute(state, operation).into_parts();
            (result, Some(report))
        }
        2 => {
            let (result, report) = executor
                .execute_with_hooks(state, operation, CasHooks::new().on_event(|_: &CasEvent| {}))
                .into_parts();
            (result, Some(report))
        }
        _ => panic!("invalid facade"),
    }
}

#[test]
fn test_sync_paths_agree_on_decisions_and_failure_precedence() {
    for path in 0..3 {
        for scenario in 0..6 {
            let state = AtomicRef::from_value(0usize);
            let counter = AtomicUsize::new(0);
            let limit = if scenario == 5 { 3 } else { 2 };
            let executor = CasExecutor::builder()
                .max_attempts(limit)
                .no_delay()
                .build()
                .expect("valid policy");
            let (result, report) = run_path(path, &executor, &state, |current| {
                let attempt = counter.fetch_add(1, Ordering::SeqCst);
                match scenario {
                    0 => CasDecision::update(*current + 1, 17),
                    1 => CasDecision::finish(17),
                    2 => CasDecision::abort("stop"),
                    3 => CasDecision::retry("again"),
                    4 => {
                        state.store(Arc::new(*current + 1));
                        CasDecision::update(*current + 1, 0)
                    }
                    5 => match attempt {
                        0 => {
                            state.store(Arc::new(7));
                            CasDecision::update(1, 0)
                        }
                        1 => CasDecision::retry("again"),
                        _ => CasDecision::finish(17),
                    },
                    _ => unreachable!(),
                }
            });
            match scenario {
                0 | 1 | 5 => {
                    let ok = result.expect("successful scenario");
                    assert_eq!(ok.is_updated(), scenario == 0);
                    assert_eq!(*ok.output(), 17);
                    assert_eq!(ok.attempts(), if scenario == 5 { 3 } else { 1 });
                    assert_eq!(
                        **ok.current(),
                        match scenario {
                            0 => 1,
                            5 => 7,
                            _ => 0,
                        }
                    );
                    if let Some(report) = report {
                        assert_eq!(report.attempts_total(), ok.attempts());
                        assert_eq!(report.conflicts(), u32::from(scenario == 5));
                        assert_eq!(report.retry_errors(), u32::from(scenario == 5));
                    }
                }
                _ => {
                    let error = result.expect_err("failure scenario");
                    assert_eq!(
                        error.kind(),
                        match scenario {
                            2 => CasErrorKind::Abort,
                            3 => CasErrorKind::RetryExhausted,
                            _ => CasErrorKind::ConflictExhausted,
                        }
                    );
                    assert_eq!(
                        error.termination(),
                        if scenario == 2 {
                            CasTermination::Aborted
                        } else {
                            CasTermination::LimitExceeded(CasLimitKind::Attempts)
                        }
                    );
                    assert_eq!(error.attempts(), if scenario == 2 { 1 } else { 2 });
                    assert_eq!(
                        error.error().copied(),
                        match scenario {
                            2 => Some("stop"),
                            3 => Some("again"),
                            _ => None,
                        }
                    );
                    assert_eq!(
                        **error.current().expect("failure snapshot"),
                        if scenario == 4 { 2 } else { 0 }
                    );
                }
            }
        }
    }
}

#[test]
fn test_sync_soft_budgets_preserve_admitted_success_and_stop_retry() {
    for path in 0..3 {
        for operation_budget in [false, true] {
            for decision in 0..3 {
                let state = AtomicRef::from_value(0usize);
                let builder = CasExecutor::<usize, &'static str>::builder().max_attempts(3);
                let executor = if operation_budget {
                    builder.max_operation_elapsed(Some(Duration::from_millis(1)))
                } else {
                    builder.max_total_elapsed(Some(Duration::from_millis(1)))
                }
                .build()
                .expect("valid soft budget");
                let (result, _) = run_path(path, &executor, &state, |_| {
                    std::thread::sleep(Duration::from_millis(5));
                    match decision {
                        0 => CasDecision::update(1, 1),
                        1 => CasDecision::finish(1),
                        _ => CasDecision::retry("late"),
                    }
                });
                if decision < 2 {
                    assert_eq!(result.expect("admitted commit is not revoked").attempts(), 1);
                    assert_eq!(*state.load(), usize::from(decision == 0));
                } else {
                    let error = result.expect_err("budget prevents second attempt");
                    assert_eq!(error.attempts(), 1);
                    assert_eq!(
                        error.kind(),
                        if operation_budget {
                            CasErrorKind::OperationBudgetExceeded
                        } else {
                            CasErrorKind::TotalBudgetExceeded
                        }
                    );
                }
            }
        }
    }
}
