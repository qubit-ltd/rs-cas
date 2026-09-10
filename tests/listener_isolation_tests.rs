// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Observation failures must not change the committed business outcome.

use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::panic::panic_any;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAlert;
use qubit_cas::CasDecision;
use qubit_cas::CasEvent;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::CasListenerKind;
use qubit_cas::ContentionThresholds;

/// Classifies events for targeted panic injection.
fn event_kind(event: &CasEvent) -> CasListenerKind {
    match event {
        CasEvent::ExecutionStarted { .. } => CasListenerKind::ExecutionStarted,
        CasEvent::AttemptFailed { .. } => CasListenerKind::AttemptFailed,
        CasEvent::RetryScheduled { .. } => CasListenerKind::RetryScheduled,
        CasEvent::ExecutionFinished { .. } => CasListenerKind::ExecutionFinished,
    }
}

/// A listener panic payload whose cleanup panics a second time.
#[derive(Debug)]
struct PanicOnDropPayload;

impl Drop for PanicOnDropPayload {
    fn drop(&mut self) {
        panic!("listener panic payload dropped");
    }
}

#[test]
fn test_each_listener_panic_is_retained_without_changing_success() {
    for kind in [
        CasListenerKind::ExecutionStarted,
        CasListenerKind::AttemptFailed,
        CasListenerKind::RetryScheduled,
        CasListenerKind::ExecutionFinished,
        CasListenerKind::ContentionAlert,
    ] {
        let state = AtomicRef::from_value(0usize);
        let counter = AtomicUsize::new(0);
        let executor = CasExecutor::<usize, ()>::builder()
            .max_attempts(2)
            .build()
            .expect("valid policy");
        let hooks = CasHooks::new()
            .on_event(move |event: &CasEvent| {
                if event_kind(event) == kind {
                    panic!("event failed");
                }
            })
            .on_contention_alert(ContentionThresholds::new(2, 1, 0.5), move |_: &CasAlert| {
                if kind == CasListenerKind::ContentionAlert {
                    panic!("alert failed");
                }
            });
        let outcome = executor.execute_with_hooks(
            &state,
            |current: &usize| {
                if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                    state.store(Arc::new(7));
                }
                CasDecision::update(*current + 1, ())
            },
            hooks,
        );
        assert!(outcome.result().is_ok());
        assert_eq!(*state.load(), 8);
        assert_eq!(outcome.report().conflicts(), 1);
        assert_eq!(outcome.report().listener_failures().len(), 1);
        assert_eq!(outcome.report().listener_failures()[0].kind(), kind);
    }
}

#[test]
fn test_all_listener_panics_preserve_order_and_finished_snapshot() {
    let state = AtomicRef::from_value(0usize);
    let counter = AtomicUsize::new(0);
    let finished_count = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&finished_count);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if let CasEvent::ExecutionFinished { report } = event {
            observed.store(report.listener_failures().len(), Ordering::SeqCst);
        }
        panic!("listener failed");
    });
    let outcome = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .build()
        .expect("valid policy")
        .execute_with_hooks(
            &state,
            |_: &usize| {
                if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                    CasDecision::retry(())
                } else {
                    CasDecision::finish(())
                }
            },
            hooks,
        );
    assert!(outcome.result().is_ok());
    assert_eq!(outcome.report().retry_errors(), 1);
    let kinds = outcome
        .report()
        .listener_failures()
        .iter()
        .map(|failure| failure.kind())
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            CasListenerKind::ExecutionStarted,
            CasListenerKind::AttemptFailed,
            CasListenerKind::RetryScheduled,
            CasListenerKind::ExecutionFinished
        ]
    );
    assert_eq!(finished_count.load(Ordering::SeqCst), 3);
}

#[test]
fn test_operation_panic_propagates_and_listener_can_reenter() {
    let state = Arc::new(AtomicRef::from_value(0usize));
    let executor = CasExecutor::<usize, ()>::builder().build().expect("valid policy");
    let panic = catch_unwind(AssertUnwindSafe(|| {
        executor.execute_result(&state, |_: &usize| -> CasDecision<usize, (), ()> {
            panic!("operation panic")
        })
    }));
    assert!(panic.is_err());
    assert_eq!(*state.load(), 0);
    let nested_executor = executor.clone();
    let nested_state = Arc::clone(&state);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if matches!(event, CasEvent::ExecutionStarted { .. }) {
            let _ = nested_executor
                .execute(&nested_state, |_: &usize| CasDecision::finish(()))
                .expect("nested execution");
        }
    });
    assert!(
        executor
            .execute_with_hooks(&state, |_: &usize| CasDecision::finish(()), hooks)
            .result()
            .is_ok()
    );
}

#[test]
fn test_listener_owned_and_non_string_panics_remain_diagnostic_only() {
    for string_payload in [true, false] {
        let hooks = CasHooks::new().on_event(move |_: &CasEvent| {
            if string_payload {
                panic_any(String::from("owned listener panic"));
            } else {
                panic_any(17u32);
            }
        });
        let executor = CasExecutor::<usize, &'static str>::builder().build().expect("executor");
        let state = AtomicRef::from_value(3usize);
        let outcome = executor.execute_with_hooks(
            &state,
            |_: &usize| CasDecision::<usize, (), _>::abort("business"),
            hooks,
        );
        assert_eq!(outcome.report().aborts(), 1);
        assert_eq!(outcome.report().outcome(), CasExecutionOutcome::ErrorAbort);
        for failure in outcome.report().listener_failures() {
            let expected = if string_payload {
                "owned listener panic"
            } else {
                "non-string panic payload"
            };
            assert_eq!(failure.message(), expected);
            assert!(failure.to_string().contains(expected));
        }
        assert_eq!(outcome.report().listener_failures().len(), 3);
        let failure = outcome
            .into_result()
            .expect_err("business abort")
            .into_last_failure()
            .expect("last failure");
        assert_eq!(failure.error(), Some(&"business"));
        assert_eq!(**failure.current(), 3);
    }
}

#[test]
fn test_execution_finished_payload_drop_panic_is_isolated() {
    let state = AtomicRef::from_value(3usize);
    let hooks = CasHooks::new().on_event(|event: &CasEvent| {
        if matches!(event, CasEvent::ExecutionFinished { .. }) {
            panic_any(PanicOnDropPayload);
        }
    });
    let executor = CasExecutor::<usize, ()>::builder().build().expect("executor");
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        executor.execute_with_hooks(&state, |current: &usize| CasDecision::update(*current + 1, ()), hooks)
    }))
    .expect("execution-finished listener payload cleanup must be isolated");

    assert!(outcome.result().is_ok());
    assert_eq!(*state.load(), 4);
    assert_eq!(outcome.report().listener_failures().len(), 1);
    assert_eq!(
        outcome.report().listener_failures()[0].kind(),
        CasListenerKind::ExecutionFinished
    );
}

#[test]
fn test_contention_alert_payload_drop_panic_is_isolated() {
    let state = AtomicRef::from_value(0usize);
    let counter = AtomicUsize::new(0);
    let hooks = CasHooks::new().on_contention_alert(ContentionThresholds::new(2, 1, 0.5), |_: &CasAlert| {
        panic_any(PanicOnDropPayload)
    });
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .build()
        .expect("executor");
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        executor.execute_with_hooks(
            &state,
            |current: &usize| {
                if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                    state.store(Arc::new(7));
                }
                CasDecision::update(*current + 1, ())
            },
            hooks,
        )
    }))
    .expect("contention-alert listener payload cleanup must be isolated");

    assert!(outcome.result().is_ok());
    assert_eq!(*state.load(), 8);
    assert_eq!(outcome.report().conflicts(), 1);
    assert_eq!(outcome.report().listener_failures().len(), 1);
    assert_eq!(
        outcome.report().listener_failures()[0].kind(),
        CasListenerKind::ContentionAlert
    );
}
