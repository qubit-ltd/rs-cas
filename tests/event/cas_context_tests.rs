// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;
use qubit_cas::constants::DEFAULT_CAS_MAX_ATTEMPTS;

use crate::support::NonCloneValue;
use crate::support::TestError;

/// Verifies success values expose the captured CAS context.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_success_context_accessors_work() {
    let state = AtomicRef::from_value(5usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_retries(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let success = executor
        .execute(&state, |current: &usize| {
            CasDecision::<usize, NonCloneValue, _>::finish(NonCloneValue {
                value: if *current == 5 { "ready" } else { "unexpected" },
            })
        })
        .expect("finish should succeed");

    assert!(!success.is_updated());
    assert_eq!(*success.current().as_ref(), 5);
    assert_eq!(success.output().value, "ready");
    assert_eq!(success.context().attempts(), 1);
    assert_eq!(success.context().current_attempt(), None);
    assert_eq!(success.context().max_attempts(), 3);
    assert_eq!(success.context().max_retries(), 2);
    let default = CasExecutor::<usize, TestError>::builder()
        .build()
        .expect("default limits");
    let default_success = default
        .execute_result(&state, |_: &usize| CasDecision::finish(()))
        .expect("default finish");
    assert_eq!(default_success.context().max_attempts(), DEFAULT_CAS_MAX_ATTEMPTS);
    assert_eq!(
        default_success.context().max_retries(),
        DEFAULT_CAS_MAX_ATTEMPTS.saturating_sub(1)
    );
    assert_eq!(success.context().max_operation_elapsed(), None);
    assert_eq!(success.context().max_total_elapsed(), None);
    assert!(success.context().total_elapsed() >= success.context().last_attempt_elapsed());
    assert_eq!(success.context().current_attempt_timeout(), None);
    assert_eq!(success.context().next_delay(), None);
}

/// Verifies context accessors remain observable for bounded executions.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_bounded_context_accessors_work() {
    let state = AtomicRef::from_value(5usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(4)
        .max_operation_elapsed(Some(Duration::from_secs(2)))
        .max_total_elapsed(Some(Duration::from_secs(3)))
        .fixed_delay(Duration::from_millis(1))
        .build()
        .expect("bounded executor should build");

    let success = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::finish(())
        })
        .expect("bounded execution should succeed");
    let context = success.context();

    assert_eq!(context.attempts(), 1);
    assert_eq!(context.current_attempt(), None);
    assert_eq!(context.max_attempts(), 4);
    assert_eq!(context.max_retries(), 3);
    assert_eq!(context.max_operation_elapsed(), Some(Duration::from_secs(2)));
    assert_eq!(context.max_total_elapsed(), Some(Duration::from_secs(3)));
    assert!(context.total_elapsed() >= context.last_attempt_elapsed());
    assert_eq!(context.current_attempt_timeout(), None);
    assert_eq!(context.next_delay(), None);
}
