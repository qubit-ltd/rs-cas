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
    FastCasState,
};

type TestDecision = FastCasDecision<u64, &'static str>;
type TestOperation = fn(u64) -> TestDecision;

fn abort_stop(_current: u64) -> TestDecision {
    FastCasDecision::abort("stop")
}

#[test]
fn test_fast_cas_error_accessors_for_abort() {
    let state = FastCasState::new(4);
    let abort_stop: TestOperation = abort_stop;
    let error = FastCas::once()
        .execute(&state, abort_stop)
        .expect_err("operation should abort");

    assert_eq!(error.current(), 4);
    assert_eq!(error.attempts(), 1);
    assert!(error.is_abort());
    assert!(!error.is_conflict());
    assert_eq!(error.error(), Some(&"stop"));
    assert_eq!(error.into_error(), Some("stop"));
}

#[test]
fn test_fast_cas_error_accessors_for_conflict() {
    let error = FastCasError::<&'static str>::Conflict {
        current: 9,
        attempts: 2,
    };

    assert_eq!(error.current(), 9);
    assert_eq!(error.attempts(), 2);
    assert!(!error.is_abort());
    assert!(error.is_conflict());
    assert_eq!(error.error(), None);
    assert_eq!(error.into_error(), None);
}
