/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_cas::{
    FastCas,
    FastCasDecision,
    FastCasState,
};

type TestDecision = FastCasDecision<usize, &'static str>;
type TestOperation = fn(usize) -> TestDecision;

fn increment(current: usize) -> TestDecision {
    FastCasDecision::update(current + 1, current + 1)
}

fn finish_current(current: usize) -> TestDecision {
    FastCasDecision::finish(current)
}

#[test]
fn test_fast_cas_success_accessors_for_update_and_finish() {
    let state = FastCasState::new(1);
    let increment: TestOperation = increment;
    let success = FastCas::once()
        .execute(&state, increment)
        .expect("update should succeed");

    assert_eq!(success.previous(), 1);
    assert_eq!(success.current(), 2);
    assert_eq!(success.output(), &2);
    assert_eq!(success.attempts(), 1);
    assert!(success.is_updated());
    assert!(!success.is_finished());
    assert_eq!(success.into_output(), 2);

    let finish_current: TestOperation = finish_current;
    let finished = FastCas::once()
        .execute(&state, finish_current)
        .expect("finish should succeed");
    assert_eq!(finished.previous(), 2);
    assert_eq!(finished.current(), 2);
    assert!(!finished.is_updated());
    assert!(finished.is_finished());
}
