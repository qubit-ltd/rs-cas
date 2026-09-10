// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAlert;
use qubit_cas::CasDecision;
use qubit_cas::CasEvent;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::ContentionThresholds;
use qubit_function::Consumer;

use crate::support::TestError;

/// Consumer used to verify `CasHooks::on_event` accepts rs-function traits.
struct EventRecorder {
    attempts: Arc<Mutex<Vec<u32>>>,
}

impl Consumer<CasEvent> for EventRecorder {
    /// Records the attempt count of a finished CAS execution.
    ///
    /// # Parameters
    /// - `event`: Lifecycle event.
    fn accept(&self, event: &CasEvent) {
        if let CasEvent::ExecutionFinished { report } = event {
            self.attempts
                .lock()
                .expect("event recorder should be lockable")
                .push(report.attempts_total());
        }
    }
}

/// Verifies hooks accept rs-function consumers and closures.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_hooks_accept_function_traits() {
    let state = AtomicRef::from_value(1usize);
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let recorded_attempts = Arc::clone(&attempts);
    let hooks = CasHooks::new().on_event(EventRecorder {
        attempts: Arc::clone(&recorded_attempts),
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let success = executor
        .execute_with_hooks(&state, |_current: &usize| CasDecision::finish(9usize), hooks)
        .expect("finish should succeed");

    assert_eq!(*success.output(), 9);
    assert_eq!(*attempts.lock().expect("success attempts should be lockable"), vec![1]);
}

#[test]
fn test_contention_registration_replaces_callback_and_thresholds() {
    let state = AtomicRef::from_value(0usize);
    let observed = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&observed);
    let hooks = CasHooks::new()
        .on_contention_alert(ContentionThresholds::new(99, 99, 1.0), |_: &CasAlert| {
            panic!("replaced callback must not run");
        })
        .on_contention_alert(ContentionThresholds::default(), move |alert: &CasAlert| {
            assert_eq!(alert.report().conflicts(), 3);
            count.fetch_add(1, Ordering::SeqCst);
        });
    let outcome = CasExecutor::<usize, ()>::builder()
        .max_attempts(3)
        .build()
        .expect("valid configuration")
        .execute_with_hooks(
            &state,
            |current: &usize| {
                state.store(Arc::new(*current + 1));
                CasDecision::update(*current + 2, ())
            },
            hooks,
        );
    assert!(outcome.is_err());
    assert_eq!(observed.load(Ordering::SeqCst), 1);
    assert!(outcome.report().listener_failures().is_empty());
}

#[test]
fn test_contention_alerts_require_registration_and_all_thresholds() {
    for registered in [false, true] {
        let state = AtomicRef::from_value(0usize);
        let observed = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&observed);
        let hooks = if registered {
            CasHooks::new().on_contention_alert(ContentionThresholds::new(4, 1, 0.3), move |_: &CasAlert| {
                count.fetch_add(1, Ordering::SeqCst);
            })
        } else {
            CasHooks::new()
        };
        let outcome = CasExecutor::<usize, ()>::builder()
            .max_attempts(3)
            .build()
            .expect("valid limits")
            .execute_with_hooks(
                &state,
                |current: &usize| {
                    state.store(Arc::new(*current + 1));
                    CasDecision::update(*current + 2, ())
                },
                hooks,
            );
        assert!(outcome.is_err());
        assert_eq!(outcome.report().conflicts(), 3);
        assert_eq!(observed.load(Ordering::SeqCst), 0);
        assert!(outcome.report().listener_failures().is_empty());
    }
}

#[test]
fn test_alert_snapshot_and_schedule_context_describe_the_same_execution() {
    let alerts = Arc::new(Mutex::new(Vec::new()));
    let scheduled = Arc::new(Mutex::new(Vec::new()));
    let seen_alerts = Arc::clone(&alerts);
    let seen_scheduled = Arc::clone(&scheduled);
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let hooks = CasHooks::new()
        .on_contention_alert(thresholds, |_: &CasAlert| panic!("replaced callback"))
        .on_contention_alert(thresholds, move |alert: &CasAlert| {
            seen_alerts.lock().expect("alerts").push(alert.clone())
        })
        .on_event(move |event: &CasEvent| {
            if let CasEvent::RetryScheduled { context, delay } = event {
                seen_scheduled.lock().expect("events").push((*context, *delay));
            }
        });
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor");
    let state = AtomicRef::from_value(0usize);
    let first = Cell::new(true);
    let outcome = executor.execute_with_hooks(
        &state,
        |current: &usize| {
            if first.replace(false) {
                state.store(Arc::new(7));
            }
            CasDecision::update(*current + 1, ())
        },
        hooks,
    );
    let report = outcome.report();
    assert!(outcome.is_ok());
    assert_eq!(report.attempts_total(), 2);
    assert_eq!(report.conflicts(), 1);
    assert_eq!(report.aborts(), 0);
    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
    assert_eq!(
        report.elapsed(),
        report.finished_at().duration_since(report.started_at())
    );
    assert!(report.listener_failures().is_empty());
    let alerts = alerts.lock().expect("alerts");
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].thresholds(), thresholds);
    assert_eq!(alerts[0].report().conflicts(), report.conflicts());
    assert_eq!(alerts[0].report().finished_at(), report.finished_at());
    let scheduled = scheduled.lock().expect("events");
    assert_eq!(scheduled.len(), 1);
    let (context, delay) = scheduled[0];
    assert_eq!(context.next_delay(), Some(delay));
    assert_eq!(delay, Duration::ZERO);
    assert!(context.total_elapsed() >= context.last_attempt_elapsed());
}
