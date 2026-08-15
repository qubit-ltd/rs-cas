// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_atomic::AtomicRef;
use qubit_cas::CasContext;
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
    assert_eq!(success.context().max_operation_elapsed(), None);
    assert_eq!(success.context().max_total_elapsed(), None);
    assert!(
        success.context().total_elapsed()
            >= success.context().last_attempt_elapsed()
    );
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
        .max_operation_elapsed(Some(std::time::Duration::from_secs(2)))
        .max_total_elapsed(Some(std::time::Duration::from_secs(3)))
        .fixed_delay(std::time::Duration::from_millis(1))
        .build()
        .expect("bounded executor should build");

    let success = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::finish(())
        })
        .expect("bounded execution should succeed");
    let context = std::hint::black_box(success.context());

    assert_eq!(std::hint::black_box(context.attempts()), 1);
    assert_eq!(std::hint::black_box(context.current_attempt()), None);
    assert_eq!(std::hint::black_box(context.max_attempts()), 4);
    assert_eq!(std::hint::black_box(context.max_retries()), 3);
    assert_eq!(
        std::hint::black_box(context.max_operation_elapsed()),
        Some(std::time::Duration::from_secs(2))
    );
    assert_eq!(
        std::hint::black_box(context.max_total_elapsed()),
        Some(std::time::Duration::from_secs(3))
    );
    assert!(
        std::hint::black_box(context.total_elapsed())
            >= context.last_attempt_elapsed()
    );
    assert_eq!(
        std::hint::black_box(context.current_attempt_timeout()),
        None
    );
    assert_eq!(std::hint::black_box(context.next_delay()), None);
}

/// Verifies context accessor functions remain callable without inlining.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_context_accessor_function_pointers_work() {
    let state = AtomicRef::from_value(5usize);
    let success = CasExecutor::<usize, TestError>::builder()
        .build()
        .expect("executor should build")
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::finish(())
        })
        .expect("execution should succeed");
    let context = success.context();

    let max_retries: fn(&CasContext) -> u32 = CasContext::max_retries;
    let total_elapsed: fn(&CasContext) -> std::time::Duration =
        CasContext::total_elapsed;
    let current_attempt: fn(&CasContext) -> Option<std::num::NonZeroU32> =
        CasContext::current_attempt;
    let last_attempt_elapsed: fn(&CasContext) -> std::time::Duration =
        CasContext::last_attempt_elapsed;
    let current_attempt_timeout: fn(
        &CasContext,
    ) -> Option<std::time::Duration> = CasContext::current_attempt_timeout;
    let next_delay: fn(&CasContext) -> Option<std::time::Duration> =
        CasContext::next_delay;

    assert_eq!(
        max_retries(&context),
        DEFAULT_CAS_MAX_ATTEMPTS.saturating_sub(1)
    );
    assert_eq!(current_attempt(&context), None);
    assert!(total_elapsed(&context) >= last_attempt_elapsed(&context));
    assert_eq!(current_attempt_timeout(&context), None);
    assert_eq!(next_delay(&context), None);
}
