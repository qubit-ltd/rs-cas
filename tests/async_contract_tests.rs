// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Async decision, deadline, cancellation and future ownership contracts.
#![cfg(feature = "tokio")]

use std::future::Future;
use std::future::pending;
use std::future::poll_fn;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Poll;
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
use qubit_cas::CasTimeoutScope;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::test as async_test;
use tokio::time::sleep;

type ExecutionResult = Result<CasSuccess<usize, usize>, CasError<usize, &'static str>>;

/// Runs a script through one of the three async facades.
async fn run_path<O, F>(
    path: u8,
    executor: &CasExecutor<usize, &'static str>,
    state: &AtomicRef<usize>,
    operation: O,
) -> (ExecutionResult, Option<CasExecutionReport>)
where
    O: Fn(Arc<usize>) -> F,
    F: Future<Output = CasDecision<usize, usize, &'static str>>,
{
    match path {
        0 => (executor.execute_async_result(state, operation).await, None),
        1 => {
            let (result, report) = executor.execute_async(state, operation).await.into_parts();
            (result, Some(report))
        }
        2 => {
            let (result, report) = executor
                .execute_async_with_hooks(state, operation, CasHooks::new().on_event(|_: &CasEvent| {}))
                .await
                .into_parts();
            (result, Some(report))
        }
        _ => panic!("invalid facade"),
    }
}

#[async_test]
async fn test_async_paths_agree_on_decisions() {
    for path in 0..3 {
        for scenario in 0..6 {
            let state = AtomicRef::from_value(0usize);
            let state_ref = &state;
            let counter = AtomicUsize::new(0);
            let counter_ref = &counter;
            let limit = if scenario == 5 { 3 } else { 2 };
            let executor = CasExecutor::builder()
                .max_attempts(limit)
                .build()
                .expect("valid policy");
            let (result, report) = run_path(path, &executor, &state, |current| async move {
                let attempt = counter_ref.fetch_add(1, Ordering::SeqCst);
                match scenario {
                    0 => CasDecision::update(*current + 1, 17),
                    1 => CasDecision::finish(17),
                    2 => CasDecision::abort("stop"),
                    3 => CasDecision::retry("again"),
                    4 => {
                        state_ref.store(Arc::new(*current + 1));
                        CasDecision::update(*current + 1, 0)
                    }
                    _ => match attempt {
                        0 => {
                            state_ref.store(Arc::new(7));
                            CasDecision::update(1, 0)
                        }
                        1 => CasDecision::retry("again"),
                        _ => CasDecision::finish(17),
                    },
                }
            })
            .await;
            if matches!(scenario, 0 | 1 | 5) {
                let ok = result.expect("successful script");
                assert_eq!(*ok.output(), 17);
                assert_eq!(ok.is_updated(), scenario == 0);
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
            } else {
                let error = result.expect_err("failure script");
                assert_eq!(
                    error.kind(),
                    match scenario {
                        2 => CasErrorKind::Abort,
                        3 => CasErrorKind::RetryExhausted,
                        _ => CasErrorKind::ConflictExhausted,
                    }
                );
                assert_eq!(error.attempts(), if scenario == 2 { 1 } else { 2 });
                assert_eq!(
                    error.termination(),
                    if scenario == 2 {
                        CasTermination::Aborted
                    } else {
                        CasTermination::LimitExceeded(CasLimitKind::Attempts)
                    }
                );
                assert_eq!(
                    error.error().copied(),
                    match scenario {
                        2 => Some("stop"),
                        3 => Some("again"),
                        _ => None,
                    }
                );
                assert_eq!(**error.current().expect("snapshot"), if scenario == 4 { 2 } else { 0 });
            }
        }
    }
}

#[async_test(start_paused = true)]
async fn test_attempt_timeout_action_and_retained_snapshot() {
    for path in 0..3 {
        for retry in [false, true] {
            let state = AtomicRef::from_value(7usize);
            let builder = CasExecutor::builder()
                .max_attempts(2)
                .attempt_timeout(Some(Duration::from_millis(10)));
            let executor = if retry {
                builder.retry_on_timeout()
            } else {
                builder.abort_on_timeout()
            }
            .build()
            .expect("valid timeouts");
            let (result, report) = run_path(path, &executor, &state, |_| async { pending().await }).await;
            let error = result.expect_err("pending operation times out");
            assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
            assert_eq!(error.attempts(), if retry { 2 } else { 1 });
            assert_eq!(
                error.termination(),
                if retry {
                    CasTermination::LimitExceeded(CasLimitKind::Attempts)
                } else {
                    CasTermination::TimedOut(CasTimeoutScope::Attempt)
                }
            );
            assert_eq!(**error.current().expect("original snapshot"), 7);
            assert_eq!(*state.load(), 7);
            if let Some(report) = report {
                assert_eq!(report.timeouts(), error.attempts());
            }
        }
    }
}

#[async_test(start_paused = true)]
async fn test_flow_timeout_during_attempt_and_backoff_preserves_precedence() {
    for path in 0..3 {
        for in_backoff in [false, true] {
            let state = AtomicRef::from_value(7usize);
            let executor = CasExecutor::builder()
                .max_attempts(3)
                .flow_timeout(Some(Duration::from_millis(10)))
                .attempt_timeout(Some(Duration::from_secs(1)))
                .fixed_delay(Duration::from_secs(1))
                .build()
                .expect("valid timeouts");
            let (result, _) = run_path(path, &executor, &state, |_| async move {
                if in_backoff {
                    CasDecision::retry("before backoff")
                } else {
                    pending().await
                }
            })
            .await;
            let error = result.expect_err("flow deadline");
            assert_eq!(error.kind(), CasErrorKind::FlowTimeout);
            assert_eq!(error.termination(), CasTermination::TimedOut(CasTimeoutScope::Flow));
            assert_eq!(error.attempts(), 1);
            assert_eq!(**error.current().expect("retained snapshot"), 7);
            assert_eq!(
                error.error().copied(),
                if in_backoff { Some("before backoff") } else { None }
            );
        }
    }
}

#[async_test(start_paused = true)]
async fn test_zero_flow_timeout_does_not_start_operation() {
    for path in 0..3 {
        let state = AtomicRef::from_value(7usize);
        let executor = CasExecutor::builder()
            .flow_timeout(Some(Duration::ZERO))
            .build()
            .expect("zero deadline");
        let (result, _) = run_path(path, &executor, &state, |_| async {
            panic!("operation must not start")
        })
        .await;
        let error = result.expect_err("zero flow budget");
        assert_eq!(error.kind(), CasErrorKind::FlowTimeout);
        assert_eq!(error.attempts(), 0);
        assert!(error.current().is_none());
    }
}

#[async_test(start_paused = true)]
async fn test_async_soft_budgets_do_not_cancel_admitted_success() {
    for path in 0..3 {
        for operation_budget in [false, true] {
            for decision in 0..3 {
                let state = AtomicRef::from_value(0usize);
                let builder = CasExecutor::builder().max_attempts(3);
                let executor = if operation_budget {
                    builder.max_operation_elapsed(Some(Duration::from_millis(5)))
                } else {
                    builder.max_total_elapsed(Some(Duration::from_millis(5)))
                }
                .build()
                .expect("valid budget");
                let (result, _) = run_path(path, &executor, &state, |_| async move {
                    sleep(Duration::from_millis(10)).await;
                    match decision {
                        0 => CasDecision::finish(1),
                        1 => CasDecision::update(1, 1),
                        _ => CasDecision::retry("late"),
                    }
                })
                .await;
                if decision < 2 {
                    let ok = result.expect("admitted success remains successful");
                    assert_eq!(ok.attempts(), 1);
                    assert_eq!(ok.is_updated(), decision == 1);
                } else {
                    let error = result.expect_err("budget prevents retry");
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

#[async_test(start_paused = true)]
async fn test_soft_total_budget_prevents_retry_after_backoff() {
    for path in 0..3 {
        let state = AtomicRef::from_value(0usize);
        let executor = CasExecutor::builder()
            .max_attempts(3)
            .max_total_elapsed(Some(Duration::from_millis(5)))
            .fixed_delay(Duration::from_millis(10))
            .build()
            .expect("valid budget");
        let (result, _) = run_path(path, &executor, &state, |_| async { CasDecision::retry("again") }).await;
        let error = result.expect_err("total budget prevents next attempt");
        assert_eq!(error.kind(), CasErrorKind::TotalBudgetExceeded);
        assert_eq!(error.attempts(), 1);
    }
}

/// Marks cancellation of an operation that was already polled.
struct CancelGuard<'a> {
    dropped: &'a AtomicBool,
}
impl Drop for CancelGuard<'_> {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

#[async_test]
async fn test_cancellation_drops_in_flight_operation_without_commit_or_finished_event() {
    for path in 0..3 {
        let state = AtomicRef::from_value(0usize);
        let executor = CasExecutor::builder().build().expect("valid policy");
        let dropped = AtomicBool::new(false);
        let finished = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&finished);
        let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
            if matches!(event, CasEvent::ExecutionFinished { .. }) {
                seen.fetch_add(1, Ordering::SeqCst);
            }
        });
        let operation = |_: Arc<usize>| async {
            let _guard = CancelGuard { dropped: &dropped };
            pending::<CasDecision<usize, usize, &'static str>>().await
        };
        let mut future = Box::pin(async {
            match path {
                0 => {
                    let _ = executor.execute_async_result(&state, operation).await;
                }
                1 => {
                    let _ = executor.execute_async(&state, operation).await;
                }
                _ => {
                    let _ = executor.execute_async_with_hooks(&state, operation, hooks).await;
                }
            }
        });
        poll_fn(|cx| {
            assert!(future.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        drop(future);
        assert!(dropped.load(Ordering::SeqCst));
        assert_eq!(*state.load(), 0);
        assert_eq!(finished.load(Ordering::SeqCst), 0);
    }
}

/// Verifies that ordinary async operations remain transferable to a runtime
/// worker.
fn require_send<T: Send>(_: T) {}

#[test]
fn test_async_futures_remain_send() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, ()>::builder().build().expect("valid policy");
    require_send(executor.execute_async_result(&state, |_| async { CasDecision::finish(()) }));
    require_send(executor.execute_async(&state, |_| async { CasDecision::finish(()) }));
    require_send(executor.execute_async_with_hooks(&state, |_| async { CasDecision::finish(()) }, CasHooks::new()));
}

#[async_test]
async fn test_async_snapshot_is_loaded_when_future_is_polled() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, ()>::builder().build().expect("valid policy");
    let future = executor.execute_async_result(&state, |current| async move { CasDecision::finish(*current) });
    state.store(Arc::new(9));
    assert_eq!(*future.await.expect("finish").output(), 9);
}

#[test]
fn test_async_operation_panics_propagate() {
    let runtime = RuntimeBuilder::new_current_thread().build().expect("runtime");
    for path in 0..3 {
        let state = AtomicRef::from_value(7usize);
        let executor = CasExecutor::builder().build().expect("executor");
        let panic = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(run_path(path, &executor, &state, |_| async {
                panic!("operation panic")
            }))
        }));
        assert!(panic.is_err());
        assert_eq!(*state.load(), 7);
    }
}

#[async_test(start_paused = true)]
async fn test_timeout_reports_attempt_snapshot_after_external_write() {
    for path in 0..3 {
        let state = AtomicRef::from_value(7usize);
        let executor = CasExecutor::builder()
            .attempt_timeout(Some(Duration::from_millis(10)))
            .abort_on_timeout()
            .build()
            .expect("executor");
        let (result, _) = run_path(path, &executor, &state, |_| async {
            state.store(Arc::new(9));
            pending().await
        })
        .await;
        let error = result.expect_err("deadline");
        assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
        assert_eq!(**error.current().expect("attempt snapshot"), 7);
        assert_eq!(*state.load(), 9);
    }
}
