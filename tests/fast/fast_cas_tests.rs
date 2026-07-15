// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::{
    FastCas,
    FastCasDecision,
    FastCasError,
    FastCasPolicy,
    FastCasState,
};
use std::cell::Cell;

type TestDecision = FastCasDecision<u64, &'static str>;
type TestOperation = fn(u64) -> TestDecision;
type TestUpdateOperation = fn(u64) -> Result<(u64, u64), &'static str>;

fn increment(current: u64) -> TestDecision {
    FastCasDecision::update(current + 1, current + 1)
}

fn finish_current(current: u64) -> TestDecision {
    FastCasDecision::finish(current)
}

fn abort_state(_current: u64) -> TestDecision {
    FastCasDecision::abort("state")
}

fn add_two(current: u64) -> Result<(u64, u64), &'static str> {
    Ok((current + 2, current + 2))
}

fn reject_update(_current: u64) -> Result<(u64, u64), &'static str> {
    Err("bad")
}

#[test]
fn test_fast_cas_constructors_and_policy_accessor() {
    assert_eq!(FastCas::once().policy(), FastCasPolicy::Once);
    assert_eq!(
        FastCas::spin(3).policy(),
        FastCasPolicy::Spin { max_attempts: 3 }
    );
    assert_eq!(
        FastCas::spin_yield(1, 3).policy(),
        FastCasPolicy::SpinYield {
            spin_attempts: 1,
            max_attempts: 3,
        }
    );
    assert_eq!(FastCas::default().policy(), FastCasPolicy::spin(16));
    assert_eq!(
        FastCas::with_policy(FastCasPolicy::once()).policy(),
        FastCasPolicy::Once
    );
}

#[test]
fn test_fast_cas_execute_updates_finishes_and_aborts() {
    let state = FastCasState::new(0);
    let cas = FastCas::once();

    let increment: TestOperation = increment;
    let success = cas
        .execute(&state, increment)
        .expect("update should succeed");
    assert_eq!(success.previous(), 0);
    assert_eq!(success.current(), 1);
    assert_eq!(success.into_output(), 1);
    assert_eq!(state.load(), 1);

    let finish_current: TestOperation = finish_current;
    let finished = cas
        .execute(&state, finish_current)
        .expect("finish should succeed");
    assert_eq!(finished.previous(), 1);
    assert_eq!(finished.current(), 1);

    let abort_state: TestOperation = abort_state;
    let error = cas
        .execute(&state, abort_state)
        .expect_err("abort should fail");
    assert!(matches!(
        error,
        FastCasError::Abort {
            current: 1,
            attempts: 1,
            ..
        }
    ));
    assert_eq!(error.into_error(), Some("state"));
}

#[test]
fn test_fast_cas_update_by_updates_or_aborts() {
    let state = FastCasState::new(2);
    let cas = FastCas::once();

    let add_two: TestUpdateOperation = add_two;
    let success = cas
        .update_by(&state, add_two)
        .expect("update should succeed");
    assert_eq!(success.previous(), 2);
    assert_eq!(success.current(), 4);
    assert_eq!(success.into_output(), 4);

    let reject_update: TestUpdateOperation = reject_update;
    let error = cas
        .update_by(&state, reject_update)
        .expect_err("operation should abort");
    assert_eq!(error.current(), 4);
    assert_eq!(error.into_error(), Some("bad"));
}

#[test]
fn test_fast_cas_compare_update_requires_expected_state() {
    let state = FastCasState::new(3);
    let cas = FastCas::spin(8);

    let wrong = cas
        .compare_update(&state, 2, 4)
        .expect_err("wrong expected state should conflict");
    assert_eq!(wrong.current(), 3);
    assert_eq!(wrong.attempts(), 1);

    let success = cas
        .compare_update(&state, 3, 4)
        .expect("expected state should update");
    assert_eq!(success.previous(), 3);
    assert_eq!(success.current(), 4);
    assert_eq!(success.into_output(), ());
}

#[test]
fn test_fast_cas_compare_update_with_runs_output_after_success() {
    let state = FastCasState::new(10);
    let cas = FastCas::once();

    let success = cas
        .compare_update_with(&state, 10, 11, |previous, current| {
            current - previous
        })
        .expect("expected state should update");

    assert_eq!(success.previous(), 10);
    assert_eq!(success.current(), 11);
    assert_eq!(success.into_output(), 1);
}

#[test]
fn test_fast_cas_execute_contention_paths() {
    let state = FastCasState::new(0);
    let mode = Cell::new(0u64);
    let operation = |current| match mode.get() {
        0 => {
            if current == 0 {
                state.store(1);
            }
            FastCasDecision::update(current + 1, current + 1)
        }
        1 => {
            state.store(current + 1);
            FastCasDecision::update(current + 2, current + 2)
        }
        2 => {
            if current == 0 {
                state.store(1);
                FastCasDecision::update(2, 2)
            } else {
                FastCasDecision::finish(1)
            }
        }
        3 => {
            if current == 0 {
                state.store(1);
                FastCasDecision::update(2, 2)
            } else {
                FastCasDecision::abort("aborted")
            }
        }
        _ => {
            if current < 2 {
                state.store(current + 1);
            }
            FastCasDecision::update(current + 1, current + 1)
        }
    };

    mode.set(0);
    let success = FastCas::spin(3)
        .execute(&state, operation)
        .expect("second attempt should succeed");
    assert_eq!(success.previous(), 1);
    assert_eq!(success.current(), 2);
    assert_eq!(success.attempts(), 2);

    state.store(0);
    mode.set(1);
    let error = FastCas::once()
        .execute(&state, operation)
        .expect_err("single attempt should conflict");
    assert!(error.is_conflict());
    assert_eq!(error.current(), 1);
    assert_eq!(error.attempts(), 1);

    state.store(0);
    let error = FastCas::spin(2)
        .execute(&state, operation)
        .expect_err("attempt budget should be exhausted");
    assert_eq!(error.current(), 2);
    assert_eq!(error.attempts(), 2);

    state.store(0);
    mode.set(2);
    let success = FastCas::spin(2)
        .execute(&state, operation)
        .expect("finish after conflict should succeed");
    assert_eq!(success.previous(), 1);
    assert_eq!(success.current(), 1);
    assert_eq!(success.attempts(), 2);
    assert_eq!(success.into_output(), 1);

    state.store(0);
    mode.set(3);
    let error = FastCas::spin(2)
        .execute(&state, operation)
        .expect_err("abort after conflict should fail");
    assert_eq!(error.current(), 1);
    assert_eq!(error.attempts(), 2);
    assert_eq!(error.into_error(), Some("aborted"));

    state.store(0);
    mode.set(4);
    let success = FastCas::spin_yield(1, 3)
        .execute(&state, operation)
        .expect("third attempt should succeed");
    assert_eq!(success.previous(), 2);
    assert_eq!(success.current(), 3);
    assert_eq!(success.attempts(), 3);
}
