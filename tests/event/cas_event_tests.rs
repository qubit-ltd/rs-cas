/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::sync::{Arc, Mutex};

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasEvent, CasExecutor, CasHooks, CasObservabilityConfig};

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
            CasEvent::RetryScheduled { .. } => "retry_scheduled",
            CasEvent::ExecutionFinished { .. } => "finished",
        };
        seen_events
            .lock()
            .expect("event vector should be lockable")
            .push(name);
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
        .build()
        .expect("executor should build");
    let state = AtomicRef::from_value(7usize);

    let success = executor
        .execute_with_hooks(
            &state,
            |_current: &usize| CasDecision::finish(11usize),
            hooks,
        )
        .expect("execution should finish");
    assert_eq!(*success.output(), 11usize);

    let events = seen.lock().expect("event vector should be lockable");
    assert!(events.iter().any(|name| *name == "started"));
    assert!(events.iter().any(|name| *name == "finished"));
}
