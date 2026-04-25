/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::error::Error;

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasErrorKind, CasExecutor};

use crate::support::TestError;

/// Verifies terminal CAS errors expose display text and source errors.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_error_display_and_source_work() {
    let state = AtomicRef::from_value(3usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let error = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::retry(TestError("still-busy"))
        })
        .expect_err("retry exhaustion should fail");

    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert!(error.to_string().contains("retryable failures exhausted"));
    assert_eq!(
        error.source().map(ToString::to_string),
        Some("still-busy".to_string())
    );
    assert_eq!(error.error(), Some(&TestError("still-busy")));
    assert_eq!(error.current().map(|current| **current), Some(3));
}
