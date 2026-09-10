// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::error::Error;
use std::sync::Arc;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAttemptFailure;
use qubit_cas::CasBoxError;
use qubit_cas::CasBuilder;
use qubit_cas::CasDecision;
use qubit_cas::CasError;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;
use qubit_cas::CasLimitKind;
use qubit_cas::CasTermination;
use qubit_error::BoxError;

#[derive(Debug)]
struct NonCloneSnapshot;

/// Checks that a public terminal error participates in standard error chains.
fn require_error<T: Error>() {}

#[test]
fn test_default_boxed_error_preserves_business_source() {
    require_error::<CasError<usize, CasBoxError>>();
    let state = AtomicRef::from_value(1usize);
    let executor: CasExecutor<usize> = CasBuilder::<usize>::new().build().expect("valid builder");
    let error = executor
        .execute_result(&state, |_: &usize| {
            CasDecision::<usize, (), _>::abort(CasBoxError::new(Box::new(std::io::Error::other("business failure"))))
        })
        .expect_err("abort must fail");
    let wrapper = error.source().expect("terminal error retains wrapper");
    assert!(wrapper.is::<CasBoxError>());
    let source = wrapper.source().expect("wrapper retains business source");
    assert!(source.is::<std::io::Error>());
    assert_eq!(source.to_string(), "business failure");

    let default_executor = CasExecutor::<usize>::builder().build().expect("valid default builder");
    let _: CasExecutor<usize, CasBoxError> = default_executor;
}

#[test]
fn test_boxed_error_conversion_retains_original_error() {
    let original: BoxError = Box::new(std::io::Error::other("original error"));
    let wrapped = CasBoxError::from(original);
    assert!(wrapped.as_inner().is::<std::io::Error>());
    assert_eq!(wrapped.to_string(), "original error");
    assert_eq!(format!("{wrapped:?}"), format!("{:?}", wrapped.as_inner()));
    let original = wrapped.into_inner();
    assert_eq!(
        original
            .downcast::<std::io::Error>()
            .expect("concrete error retained")
            .to_string(),
        "original error"
    );
}

#[test]
fn test_error_clone_does_not_require_snapshot_clone() {
    let state = AtomicRef::from_value(NonCloneSnapshot);
    let error: CasError<NonCloneSnapshot, String> = CasExecutor::<NonCloneSnapshot, String>::builder()
        .build()
        .expect("valid builder")
        .execute_result(&state, |_: &NonCloneSnapshot| {
            CasDecision::<_, (), _>::abort("stopped".to_owned())
        })
        .expect_err("abort must fail");
    let cloned = error.clone();
    assert_eq!(cloned.kind(), error.kind());
    assert_eq!(cloned.context(), error.context());
    assert_eq!(cloned.error(), error.error());
    assert!(Arc::ptr_eq(
        cloned.current().expect("retained snapshot"),
        error.current().expect("retained snapshot")
    ));
}

#[test]
fn test_attempt_failure_clone_preserves_variants_and_snapshot_identity() {
    let current = Arc::new(NonCloneSnapshot);
    let failures: [CasAttemptFailure<NonCloneSnapshot, String>; 4] = [
        CasAttemptFailure::Conflict {
            current: Arc::clone(&current),
        },
        CasAttemptFailure::Retry {
            current: Arc::clone(&current),
            error: "retry".to_owned(),
        },
        CasAttemptFailure::Abort {
            current: Arc::clone(&current),
            error: "abort".to_owned(),
        },
        CasAttemptFailure::Timeout {
            current: Arc::clone(&current),
        },
    ];
    for failure in failures {
        let cloned = failure.clone();
        assert_eq!(std::mem::discriminant(&cloned), std::mem::discriminant(&failure));
        assert!(Arc::ptr_eq(cloned.current(), &current));
        assert_eq!(cloned.error(), failure.error());
    }
}

#[test]
fn test_abort_preserves_business_error() {
    let state = AtomicRef::from_value(1usize);
    let error = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(3)
        .no_delay()
        .build()
        .expect("valid builder")
        .execute_result(&state, |_current: &usize| CasDecision::<usize, (), &str>::abort("bad"))
        .expect_err("abort must fail");
    assert_eq!(error.kind(), CasErrorKind::Abort);
    assert_eq!(error.termination(), CasTermination::Aborted);
    assert_eq!(error.error(), Some(&"bad"));
}

#[test]
fn test_retry_exhaustion_has_domain_termination() {
    let state = AtomicRef::from_value(1usize);
    let error = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(1)
        .no_delay()
        .build()
        .expect("valid builder")
        .execute_result(&state, |_current: &usize| CasDecision::<usize, (), &str>::retry("busy"))
        .expect_err("retry must fail");
    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(
        error.termination(),
        CasTermination::LimitExceeded(CasLimitKind::Attempts)
    );
}
