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
use qubit_cas::{CasAttemptFailure, CasContext, CasDecision, CasHooks, CasSuccess};
use qubit_function::Consumer;

use crate::support::TestError;

/// Consumer used to verify `CasHooks::on_success` accepts rs-function traits.
struct SuccessRecorder {
    attempts: Arc<Mutex<Vec<u32>>>,
}

impl Consumer<CasSuccess<usize, usize>> for SuccessRecorder {
    /// Records the attempt count of a successful CAS execution.
    ///
    /// # Parameters
    /// - `success`: Successful CAS value.
    fn accept(&self, success: &CasSuccess<usize, usize>) {
        self.attempts
            .lock()
            .expect("success recorder should be lockable")
            .push(success.attempts());
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
    let retry_attempts = Arc::new(Mutex::new(Vec::new()));
    let retry_events = Arc::clone(&retry_attempts);

    let hooks = CasHooks::new()
        .on_success(SuccessRecorder {
            attempts: Arc::clone(&recorded_attempts),
        })
        .on_retry(
            move |context: &CasContext, failure: &CasAttemptFailure<usize, TestError>| {
                retry_events
                    .lock()
                    .expect("retry events should be lockable")
                    .push((context.attempt(), failure.is_conflict()));
            },
        );

    let executor = qubit_cas::CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
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
        retry_attempts
            .lock()
            .expect("retry attempts should be lockable")
            .is_empty()
    );
}
