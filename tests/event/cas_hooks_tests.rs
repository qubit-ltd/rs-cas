// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::{
    Arc,
    Mutex,
};

use qubit_atomic::AtomicRef;
use qubit_cas::{
    CasDecision,
    CasEvent,
    CasHooks,
    CasObservabilityConfig,
};
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

    let executor = qubit_cas::CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
        .build()
        .expect("executor should build");

    let success = executor
        .execute_with_hooks(
            &state,
            |_current: &usize| CasDecision::finish(9usize),
            hooks,
        )
        .expect("finish should succeed");

    assert_eq!(*success.output(), 9);
    assert_eq!(
        *attempts
            .lock()
            .expect("success attempts should be lockable"),
        vec![1]
    );
    assert!(
        attempts
            .lock()
            .expect("success attempts should be lockable")
            .len()
            == 1
    );
}
