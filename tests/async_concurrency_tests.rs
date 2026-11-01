// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Deterministic interleaving and per-execution isolation of async CAS calls.

#![cfg(feature = "tokio")]

use std::future::Future;
use std::future::pending;
use std::future::poll_fn;
use std::sync::Arc;
use std::sync::Mutex;
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
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutionReport;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::CasSuccess;
use qubit_cas::CasTermination;
use qubit_cas::CasTimeoutScope;
use tokio::sync::Notify;
use tokio::test as async_test;
use tokio::time::advance;

type ExecutionResult = Result<CasSuccess<usize, usize>, CasError<usize, &'static str>>;
type EventLog = Arc<Mutex<Vec<&'static str>>>;

/// Runs one facade with a log owned exclusively by that execution.
async fn run_path<O, F>(
    path: u8,
    executor: &CasExecutor<usize, &'static str>,
    state: &AtomicRef<usize>,
    operation: O,
    log: EventLog,
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
            let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
                let name = match event {
                    CasEvent::ExecutionStarted { .. } => "started",
                    CasEvent::AttemptFailed { .. } => "failed",
                    CasEvent::RetryScheduled { .. } => "scheduled",
                    CasEvent::ExecutionFinished { .. } => "finished",
                };
                log.lock().expect("event log").push(name);
            });
            let (result, report) = executor
                .execute_async_with_hooks(state, operation, hooks)
                .await
                .into_parts();
            (result, Some(report))
        }
        _ => panic!("invalid facade"),
    }
}

/// Marks destruction of an operation that was actually polled.
struct OperationDrop<'a>(&'a AtomicBool);

impl Drop for OperationDrop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[async_test]
async fn test_interleaved_async_updates_recompute_after_conflict() {
    for path in 0..3 {
        let state = AtomicRef::from_value(0usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .max_attempts(2)
            .no_delay()
            .build()
            .expect("valid policy");
        let cloned = executor.clone();
        let gate_a = Notify::new();
        let gate_b = Notify::new();
        let calls_a = AtomicUsize::new(0);
        let calls_b = AtomicUsize::new(0);
        let log_a: EventLog = Arc::new(Mutex::new(Vec::new()));
        let log_b: EventLog = Arc::new(Mutex::new(Vec::new()));
        let operation_a = |current: Arc<usize>| {
            let first = calls_a.fetch_add(1, Ordering::SeqCst) == 0;
            let gate = &gate_a;
            async move {
                if first {
                    gate.notified().await;
                }
                CasDecision::update(*current + 1, *current)
            }
        };
        let operation_b = |current: Arc<usize>| {
            let first = calls_b.fetch_add(1, Ordering::SeqCst) == 0;
            let gate = &gate_b;
            async move {
                if first {
                    gate.notified().await;
                }
                CasDecision::update(*current + 1, *current)
            }
        };
        let mut a = Box::pin(run_path(path, &executor, &state, operation_a, Arc::clone(&log_a)));
        let mut b = Box::pin(run_path(path, &cloned, &state, operation_b, Arc::clone(&log_b)));
        poll_fn(|cx| {
            assert!(a.as_mut().poll(cx).is_pending());
            assert!(b.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert_eq!(calls_b.load(Ordering::SeqCst), 1);
        assert_eq!(*state.load(), 0);
        gate_a.notify_one();
        let (a_result, a_report) = a.await;
        let a_success = a_result.expect("first writer commits");
        assert_eq!(*state.load(), 1);
        gate_b.notify_one();
        let (b_result, b_report) = b.await;
        let b_success = b_result.expect("second writer recomputes");
        assert_eq!(a_success.attempts(), 1);
        assert_eq!(b_success.attempts(), 2);
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert_eq!(calls_b.load(Ordering::SeqCst), 2);
        assert_eq!(**a_success.previous().expect("update"), 0);
        assert_eq!(**a_success.current(), 1);
        assert_eq!(*a_success.output(), 0);
        assert_eq!(**b_success.previous().expect("update"), 1);
        assert_eq!(**b_success.current(), 2);
        assert_eq!(*b_success.output(), 1);
        assert!(Arc::ptr_eq(a_success.current(), b_success.previous().expect("update")));
        assert!(Arc::ptr_eq(b_success.current(), &state.load()));
        assert_eq!(*state.load(), 2);
        for (report, attempts, conflicts) in [(a_report, 1, 0), (b_report, 2, 1)] {
            if let Some(report) = report {
                assert_eq!(report.attempts_total(), attempts);
                assert_eq!(report.conflicts(), conflicts);
                assert_eq!(report.retry_errors(), 0);
                assert_eq!(report.aborts(), 0);
                assert_eq!(report.timeouts(), 0);
                assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
                assert!(report.listener_failures().is_empty());
            }
        }
        if path == 2 {
            assert_eq!(*log_a.lock().expect("A events"), vec!["started", "finished"]);
            assert_eq!(
                *log_b.lock().expect("B events"),
                vec!["started", "failed", "scheduled", "finished"]
            );
        } else {
            assert!(log_a.lock().expect("A events").is_empty());
            assert!(log_b.lock().expect("B events").is_empty());
        }
    }
}

#[async_test(start_paused = true)]
async fn test_async_timeout_and_cancellation_remain_execution_local() {
    for path in 0..3 {
        for cancel in [false, true] {
            let state = AtomicRef::from_value(0usize);
            let executor = CasExecutor::<usize, &'static str>::builder()
                .max_attempts(2)
                .no_delay()
                .attempt_timeout(Some(Duration::from_millis(10)))
                .build()
                .expect("valid timeout");
            let cloned = executor.clone();
            let dropped = AtomicBool::new(false);
            let gate = Notify::new();
            let slow_log: EventLog = Arc::new(Mutex::new(Vec::new()));
            let fast_log: EventLog = Arc::new(Mutex::new(Vec::new()));
            let mut slow = Box::pin(run_path(
                path,
                &executor,
                &state,
                |_| async {
                    let _guard = OperationDrop(&dropped);
                    pending::<CasDecision<usize, usize, &'static str>>().await
                },
                Arc::clone(&slow_log),
            ));
            let fast_operation = |current: Arc<usize>| {
                let gate = &gate;
                async move {
                    gate.notified().await;
                    CasDecision::update(*current + 1, *current)
                }
            };
            let mut fast = Box::pin(run_path(path, &cloned, &state, fast_operation, Arc::clone(&fast_log)));
            poll_fn(|cx| {
                assert!(slow.as_mut().poll(cx).is_pending());
                assert!(fast.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            assert_eq!(*state.load(), 0);
            assert!(!dropped.load(Ordering::SeqCst));
            gate.notify_one();
            let (fast_result, fast_report) = fast.await;
            let success = fast_result.expect("other execution commits independently");
            assert_eq!(success.attempts(), 1);
            assert_eq!(**success.previous().expect("update"), 0);
            assert_eq!(**success.current(), 1);
            assert_eq!(*success.output(), 0);
            assert!(!dropped.load(Ordering::SeqCst));
            if cancel {
                drop(slow);
            } else {
                advance(Duration::from_millis(11)).await;
                let (result, report) = slow.await;
                let error = result.expect_err("only the slow execution times out");
                assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
                assert_eq!(error.termination(), CasTermination::TimedOut(CasTimeoutScope::Attempt));
                assert_eq!(error.attempts(), 1);
                assert_eq!(**error.current().expect("original slow snapshot"), 0);
                assert!(error.error().is_none());
                if let Some(report) = report {
                    assert_eq!(report.attempts_total(), 1);
                    assert_eq!(report.timeouts(), 1);
                    assert_eq!(report.conflicts(), 0);
                    assert_eq!(report.retry_errors(), 0);
                    assert_eq!(report.aborts(), 0);
                    assert_eq!(report.outcome(), CasExecutionOutcome::ErrorAttemptTimeout);
                    assert!(report.listener_failures().is_empty());
                }
            }
            assert!(dropped.load(Ordering::SeqCst));
            assert_eq!(*state.load(), 1);
            assert!(Arc::ptr_eq(success.current(), &state.load()));
            if let Some(report) = fast_report {
                assert_eq!(report.attempts_total(), 1);
                assert_eq!(report.timeouts(), 0);
                assert_eq!(report.conflicts(), 0);
                assert_eq!(report.retry_errors(), 0);
                assert_eq!(report.aborts(), 0);
                assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
                assert!(report.listener_failures().is_empty());
            }
            if path == 2 {
                assert_eq!(*fast_log.lock().expect("fast events"), vec!["started", "finished"]);
                let expected = if cancel {
                    vec!["started"]
                } else {
                    vec!["started", "failed", "finished"]
                };
                assert_eq!(*slow_log.lock().expect("slow events"), expected);
            } else {
                assert!(fast_log.lock().expect("fast events").is_empty());
                assert!(slow_log.lock().expect("slow events").is_empty());
            }
        }
    }
}
