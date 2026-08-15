// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_atomic::AtomicRef;
use qubit_cas::CasAttemptFailure;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;
use qubit_cas::CasRetryFailure;

use crate::support::TestError;

/// Verifies heap-owned CAS error details preserve every public terminal part.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_error_details_preserve_owned_terminal_parts() {
    let state = AtomicRef::from_value(3usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let error = executor
        .execute_result(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::abort(TestError("blocked"))
        })
        .expect_err("abort should fail");

    let (kind, failure, context, last_failure) = error.into_parts();
    assert_eq!(kind, CasErrorKind::Abort);
    assert_eq!(failure, CasRetryFailure::Aborted);
    assert_eq!(context.attempts(), 1);
    match last_failure {
        Some(CasAttemptFailure::Abort { current, error }) => {
            assert_eq!(*current, 3);
            assert_eq!(error, TestError("blocked"));
        }
        other => panic!("expected an owned abort failure, got {other:?}"),
    }
}
