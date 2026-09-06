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
use qubit_cas::CasAttemptFailureKind;
use qubit_cas::CasDecision;
use qubit_cas::CasEvent;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::CasObservabilityConfig;

use crate::support::TestError;

/// Verifies event stream emits both start and finish events.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_event_stream_emits_started_and_finished() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_events = Arc::clone(&seen);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        let name = match event {
            CasEvent::ExecutionStarted { .. } => "started",
            CasEvent::AttemptFailed { .. } => "attempt_failed",
            CasEvent::RetryRequested { .. } => "retry_requested",
            CasEvent::ExecutionFinished { .. } => "finished",
        };
        seen_events.lock().expect("event vector should be lockable").push(name);
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
        .build()
        .expect("executor should build");
    let state = AtomicRef::from_value(7usize);

    let success = executor
        .execute_with_hooks(&state, |_current: &usize| CasDecision::finish(11usize), hooks)
        .expect("execution should finish");
    assert_eq!(*success.output(), 11usize);

    let events = seen.lock().expect("event vector should be lockable");
    assert!(events.contains(&"started"));
    assert!(events.contains(&"finished"));
}

/// Verifies event stream emits retry-requested events for retryable CAS
/// failures.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_event_stream_emits_retry_requested_for_conflict() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_events = Arc::clone(&seen);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if let CasEvent::RetryRequested { context } = event {
            seen_events
                .lock()
                .expect("event vector should be lockable")
                .push(context.attempts());
        }
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
        .build()
        .expect("executor should build");

    let success = executor
        .execute_with_hooks(
            &state,
            |current: &usize| {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, ())
            },
            hooks,
        )
        .expect("second attempt should succeed");

    assert_eq!(success.attempts(), 2);
    assert_eq!(*seen.lock().expect("event vector should be lockable"), vec![1]);
}

/// Verifies a final-attempt conflict still emits the retry intent while the
/// report and lifecycle events count only the one admitted attempt.
#[test]
fn test_event_stream_preserves_final_attempt_retry_intent() {
    let state = AtomicRef::from_value(0usize);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_events = Arc::clone(&seen);
    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        let name = match event {
            CasEvent::ExecutionStarted { .. } => "started",
            CasEvent::AttemptFailed { kind, .. } => {
                assert_eq!(*kind, CasAttemptFailureKind::Conflict);
                "attempt_failed"
            }
            CasEvent::RetryRequested { .. } => "retry_requested",
            CasEvent::ExecutionFinished { .. } => "finished",
        };
        seen_events.lock().expect("event vector should be lockable").push(name);
    });
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(1)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
        .build()
        .expect("executor should build");

    let outcome = executor.execute_with_hooks(
        &state,
        |current: &usize| {
            state.store(Arc::new(*current + 1));
            CasDecision::update(*current + 1, ())
        },
        hooks,
    );
    let report = outcome.report().clone();
    let _error = outcome.expect_err("the only admitted attempt should conflict");

    assert_eq!(report.attempts_total(), 1);
    let events = seen.lock().expect("event vector should be lockable");
    assert_eq!(events.iter().filter(|event| **event == "started").count(), 1);
    assert_eq!(events.iter().filter(|event| **event == "attempt_failed").count(), 1);
    assert_eq!(events.iter().filter(|event| **event == "retry_requested").count(), 1);
    assert_eq!(events.iter().filter(|event| **event == "finished").count(), 1);
}
