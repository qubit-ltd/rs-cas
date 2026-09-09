// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Executor reuse and builder override contracts.

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

struct State {
    value: usize,
}
struct Failure;

#[test]
fn test_clone_executor_without_clone_state_or_error() {
    let executor = CasExecutor::<State, Failure>::builder().build().expect("valid policy");
    let cloned = executor.clone();
    let state = AtomicRef::from_value(State { value: 3 });
    let ok = cloned
        .execute_result(&state, |s: &State| CasDecision::finish(s.value))
        .expect("finish succeeds");
    assert_eq!(*ok.output(), 3);
}

/// Compile-time assertion for reusable executor configurations.
fn require_send_sync<T: Send + Sync>(_: &T) {}

#[test]
fn test_executor_remains_send_sync_and_shared_between_threads() {
    let executor = CasExecutor::<State, Failure>::builder().build().expect("valid policy");
    require_send_sync(&executor);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let executor = &executor;
            scope.spawn(move || {
                let state = AtomicRef::from_value(State { value: 8 });
                let ok = executor
                    .execute_result(&state, |s: &State| CasDecision::finish(s.value))
                    .expect("finish");
                assert_eq!(*ok.output(), 8);
            });
        }
    });
}

#[test]
fn test_strategy_and_individual_settings_follow_call_order() {
    use std::time::Duration;

    use qubit_cas::CasStrategy;
    for strategy_last in [false, true] {
        let builder = CasExecutor::<usize, ()>::builder();
        let builder = if strategy_last {
            builder.max_attempts(1).strategy(CasStrategy::ContentionBackoff)
        } else {
            builder.strategy(CasStrategy::ContentionBackoff).max_attempts(1)
        };
        let executor = builder
            .max_operation_elapsed(None)
            .max_total_elapsed(None)
            .no_delay()
            .build()
            .expect("valid policy");
        let state = AtomicRef::from_value(0usize);
        let error = executor
            .execute_result(&state, |_: &usize| CasDecision::<usize, (), ()>::retry(()))
            .expect_err("retry exhausts");
        assert_eq!(error.attempts(), if strategy_last { 64 } else { 1 });
    }
    let executor = CasExecutor::<usize, ()>::builder()
        .attempt_timeout(Some(Duration::from_secs(1)))
        .flow_timeout(Some(Duration::from_secs(2)))
        .strategy(CasStrategy::ContentionBackoff)
        .build()
        .expect("valid timeout policy");
    assert_eq!(executor.attempt_timeout(), Some(Duration::from_secs(1)));
    assert_eq!(executor.flow_timeout(), Some(Duration::from_secs(2)));
}

#[test]
fn test_no_delay_replaces_fixed_delay_and_zero_fixed_is_equivalent() {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    let (sender, receiver) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        for mode in 0..3 {
            let builder = CasExecutor::<usize, ()>::builder().max_attempts(2);
            let executor = match mode {
                0 => builder.fixed_delay(Duration::from_secs(30)).no_delay(),
                1 => builder.no_delay(),
                _ => builder.fixed_delay(Duration::ZERO),
            }
            .build()
            .expect("valid policy");
            let counter = AtomicUsize::new(0);
            let state = AtomicRef::from_value(0usize);
            let ok = executor
                .execute_result(&state, |_: &usize| {
                    if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                        CasDecision::retry(())
                    } else {
                        CasDecision::finish(17)
                    }
                })
                .expect("second attempt succeeds");
            assert_eq!(ok.attempts(), 2);
            assert_eq!(*ok.output(), 17);
        }
        sender.send(()).expect("test receiver alive");
    });
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("no-delay must not sleep for 30 seconds");
    writer.join().expect("writer succeeds");
}
