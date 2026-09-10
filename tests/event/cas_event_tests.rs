// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Scheduling events describe admission decisions, not executed attempt counts.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasEvent;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;

#[test]
fn test_retry_scheduled_requires_remaining_attempts() {
    for limit in [1, 2] {
        let events = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&events);
        let attempts = AtomicUsize::new(0);
        let state = AtomicRef::from_value(0usize);
        let executor = CasExecutor::<usize, ()>::builder()
            .max_attempts(limit)
            .no_delay()
            .build()
            .unwrap();
        let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
            let name = match event {
                CasEvent::ExecutionStarted { .. } => "started",
                CasEvent::AttemptFailed { .. } => "failed",
                CasEvent::RetryScheduled { .. } => "scheduled",
                CasEvent::ExecutionFinished { .. } => "finished",
            };
            recorded.lock().unwrap().push(name);
        });
        let result = executor.execute_with_hooks(
            &state,
            |_: &usize| {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    CasDecision::retry(())
                } else {
                    CasDecision::finish(())
                }
            },
            hooks,
        );
        let expected = if limit == 1 {
            vec!["started", "failed", "finished"]
        } else {
            vec!["started", "failed", "scheduled", "finished"]
        };
        assert_eq!(*events.lock().unwrap(), expected);
        assert_eq!(result.report().attempts_total(), limit);
        assert_eq!(result.is_ok(), limit == 2);
    }
}

#[test]
fn test_budget_rejection_does_not_emit_retry_scheduled() {
    let state = AtomicRef::from_value(7usize);
    let scheduled = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&scheduled);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if matches!(event, CasEvent::RetryScheduled { .. }) {
            observed.fetch_add(1, Ordering::SeqCst);
        }
    });
    let outcome = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(3)
        .max_total_elapsed(Some(Duration::from_secs(5)))
        .fixed_delay(Duration::from_secs(10))
        .build()
        .unwrap()
        .execute_with_hooks(
            &state,
            |_: &usize| CasDecision::<usize, (), &'static str>::retry("busy"),
            hooks,
        );
    let error = outcome.result().as_ref().expect_err("budget rejects backoff");
    assert_eq!(error.kind(), CasErrorKind::TotalBudgetExceeded);
    assert_eq!(error.attempts(), 1);
    assert_eq!(error.error(), Some(&"busy"));
    assert_eq!(scheduled.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_scheduled_retry_can_still_end_before_another_attempt() {
    let state = AtomicRef::from_value(7usize);
    let scheduled = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&scheduled);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if matches!(event, CasEvent::RetryScheduled { .. }) {
            observed.fetch_add(1, Ordering::SeqCst);
        }
    });
    let outcome = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(3)
        .flow_timeout(Some(Duration::from_millis(10)))
        .fixed_delay(Duration::from_secs(1))
        .build()
        .unwrap()
        .execute_async_with_hooks(
            &state,
            |_| async { CasDecision::<usize, (), &'static str>::retry("busy") },
            hooks,
        )
        .await;
    let error = outcome.result().as_ref().expect_err("flow deadline during backoff");
    assert_eq!(error.kind(), CasErrorKind::FlowTimeout);
    assert_eq!(error.attempts(), 1);
    assert_eq!(error.error(), Some(&"busy"));
    assert_eq!(**error.current().expect("retained failure snapshot"), 7);
    assert_eq!(scheduled.load(Ordering::SeqCst), 1);
}
