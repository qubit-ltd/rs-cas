// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! End-to-end contracts for preset configuration and observable diagnostics.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use qubit_atomic::AtomicRef;

use crate::CasAlert;
use crate::CasBuilder;
use crate::CasDecision;
use crate::CasEvent;
use crate::CasExecutionOutcome;
use crate::CasExecutor;
use crate::CasHooks;
use crate::CasStrategy;
use crate::ContentionThresholds;

#[test]
fn test_all_preset_constructors_install_the_advertised_profile() {
    let state = AtomicRef::from_value(0usize);
    for strategy in [
        CasStrategy::LatencyFirst,
        CasStrategy::ContentionBackoff,
        CasStrategy::ReliabilityFirst,
    ] {
        let profile = strategy.profile();
        let direct = match strategy {
            CasStrategy::LatencyFirst => CasExecutor::latency_first(),
            CasStrategy::ContentionBackoff => CasExecutor::contention_backoff(),
            CasStrategy::ReliabilityFirst => CasExecutor::reliability_first(),
        };
        let built = match strategy {
            CasStrategy::LatencyFirst => CasBuilder::default().build_latency_first(),
            CasStrategy::ContentionBackoff => CasBuilder::default().build_contention_backoff(),
            CasStrategy::ReliabilityFirst => CasBuilder::default().build_reliability_first(),
        }
        .expect("preset is valid");
        for executor in [direct, built, CasExecutor::<usize, ()>::with_strategy(strategy)] {
            let outcome = executor.execute(&state, |_: &usize| CasDecision::finish(()));
            let report = outcome.report();
            assert_eq!(report.max_attempts(), profile.max_attempts());
            assert_eq!(report.max_operation_elapsed(), Some(profile.max_operation_elapsed()));
            assert_eq!(report.max_total_elapsed(), profile.max_total_elapsed());
            assert_eq!(report.outcome(), CasExecutionOutcome::SuccessFinished);
            assert!(format!("{executor:?}").contains("CasExecutor"));
        }
        assert_eq!(profile.uses_backoff(), strategy != CasStrategy::LatencyFirst);
    }
}

#[test]
fn test_random_delay_validation_reports_the_invalid_field() {
    let invalid = CasExecutor::<usize, ()>::builder()
        .random_delay(Duration::from_secs(1), Duration::ZERO)
        .build()
        .expect_err("reversed delay bounds");
    assert!(!invalid.message().is_empty());
    assert!(invalid.to_string().contains(invalid.field()));
    assert!(invalid.to_string().contains(invalid.message()));
    let executor = CasExecutor::<usize, ()>::builder()
        .random_delay(Duration::ZERO, Duration::ZERO)
        .build()
        .expect("zero delay is valid");
    let state = AtomicRef::from_value(0usize);
    assert_eq!(
        executor
            .execute_result(&state, |_: &usize| CasDecision::finish(3))
            .expect("finish")
            .output(),
        &3
    );
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
        .on_alert(move |alert: &CasAlert| seen_alerts.lock().expect("alerts").push(alert.clone()))
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

#[test]
fn test_listener_owned_and_non_string_panics_remain_diagnostic_only() {
    for string_payload in [true, false] {
        let hooks = CasHooks::new().on_event(move |_: &CasEvent| {
            if string_payload {
                std::panic::panic_any(String::from("owned listener panic"));
            } else {
                std::panic::panic_any(17u32);
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
