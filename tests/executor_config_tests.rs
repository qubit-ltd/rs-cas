// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Executor reuse and builder override contracts.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::thread::scope;
use std::thread::spawn;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasBuilder;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;
use qubit_cas::CasStrategy;

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
    scope(|scope| {
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
fn test_executor_getters_preserve_installed_limits() {
    let executor = CasExecutor::<u8, ()>::builder()
        .max_attempts(7)
        .max_operation_elapsed(Some(Duration::from_millis(3)))
        .max_total_elapsed(Some(Duration::from_millis(9)))
        .no_delay()
        .build()
        .expect("valid policy");
    assert_eq!(executor.max_attempts(), 7);
    assert_eq!(executor.max_retries(), 6);
    assert_eq!(executor.max_operation_elapsed(), Some(Duration::from_millis(3)));
    assert_eq!(executor.max_total_elapsed(), Some(Duration::from_millis(9)));
    let cloned = executor.clone();
    assert_eq!(cloned.max_attempts(), executor.max_attempts());
    assert_eq!(cloned.max_retries(), executor.max_retries());
    assert_eq!(cloned.max_operation_elapsed(), executor.max_operation_elapsed());
    assert_eq!(cloned.max_total_elapsed(), executor.max_total_elapsed());
}

#[test]
fn test_executor_getters_distinguish_defaults_presets_and_overrides() {
    let default = CasExecutor::<usize, ()>::builder().build().expect("defaults");
    assert_eq!((default.max_attempts(), default.max_retries()), (5, 4));
    assert_eq!(default.max_operation_elapsed(), None);
    assert_eq!(default.max_total_elapsed(), None);
    let preset = CasExecutor::<usize, ()>::latency_first();
    assert_eq!((preset.max_attempts(), preset.max_retries()), (100, 99));
    assert_eq!(preset.max_operation_elapsed(), Some(Duration::from_millis(5)));
    assert_eq!(preset.max_total_elapsed(), Some(Duration::from_millis(20)));
    let overridden = CasExecutor::<usize, ()>::builder()
        .strategy(CasStrategy::LatencyFirst)
        .max_attempts(1)
        .build()
        .expect("override");
    assert_eq!((overridden.max_attempts(), overridden.max_retries()), (1, 0));
}

#[test]
fn test_no_delay_replaces_fixed_delay_and_zero_fixed_is_equivalent() {
    let (sender, receiver) = channel();
    let writer = spawn(move || {
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
