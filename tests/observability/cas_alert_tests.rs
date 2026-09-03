// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAlert;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::CasObservabilityConfig;
use qubit_cas::CasObservabilityMode;
use qubit_cas::ContentionThresholds;

use crate::support::TestError;

/// Verifies contention alerts expose report and threshold snapshots.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_alert_exposes_report_and_thresholds() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let alerts = Arc::new(Mutex::new(Vec::new()));
    let alert_events = Arc::clone(&alerts);
    let hooks = CasHooks::new().on_alert(move |alert: &CasAlert| {
        alert_events.lock().expect("alert events should be lockable").push((
            alert.report().attempts_total(),
            alert.report().conflicts(),
            alert.thresholds(),
        ));
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream_with_alert(thresholds))
        .build()
        .expect("executor should build");

    let outcome = executor.execute_with_hooks(
        &state,
        |current: &usize| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                state.store(Arc::new(*current + 1));
            }
            CasDecision::update(*current + 1, ())
        },
        hooks,
    );
    let _success = outcome.expect("second attempt should succeed");

    assert_eq!(
        *alerts.lock().expect("alert events should be lockable"),
        vec![(2, 1, thresholds)]
    );
    assert_eq!(
        executor.observability().mode(),
        CasObservabilityMode::EventStreamWithAlert
    );
}

/// Verifies the alert threshold accessor can be used as a function pointer.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_alert_threshold_accessor_function_pointer() {
    let state = AtomicRef::from_value(0usize);
    let thresholds = ContentionThresholds::new(1, 0, 0.0);
    let alerts = Arc::new(Mutex::new(Vec::new()));
    let alert_events = Arc::clone(&alerts);
    let hooks = CasHooks::new().on_alert(move |alert: &CasAlert| {
        let thresholds_accessor: fn(&CasAlert) -> ContentionThresholds = CasAlert::thresholds;
        alert_events
            .lock()
            .expect("alert list should be lockable")
            .push(thresholds_accessor(alert));
    });

    CasExecutor::<usize, TestError>::builder()
        .max_attempts(1)
        .observability(CasObservabilityConfig::event_stream_with_alert(thresholds))
        .build()
        .expect("executor should build")
        .execute_with_hooks(
            &state,
            |_current: &usize| CasDecision::<usize, (), TestError>::finish(()),
            hooks,
        )
        .expect("execution should succeed");

    assert_eq!(*alerts.lock().expect("alert list should be lockable"), vec![thresholds]);
}
