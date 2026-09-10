// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#[cfg(feature = "tokio")]
use std::sync::Arc;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
#[cfg(feature = "tokio")]
use qubit_cas::CasErrorKind;
#[cfg(feature = "tokio")]
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;
#[cfg(feature = "tokio")]
use tokio::test as async_test;
#[cfg(feature = "tokio")]
use tokio::time::sleep;

#[test]
fn test_builder_returns_cas_build_error() {
    let error = CasExecutor::<usize, ()>::builder()
        .max_attempts(0)
        .build()
        .expect_err("zero attempts must be rejected");
    assert_eq!(error.field(), "retry_policy");
    assert!(error.message().contains("attempt"));
}

#[cfg(feature = "tokio")]
#[async_test(start_paused = true)]
async fn test_flow_timeout_is_hard_and_distinct_from_total_budget() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .flow_timeout(Some(Duration::from_millis(1)))
        .no_delay()
        .build()
        .expect("valid builder");
    let outcome = executor
        .execute_async_with_hooks(
            &state,
            |_current: Arc<usize>| async {
                sleep(Duration::from_secs(60)).await;
                CasDecision::<usize, (), ()>::retry(())
            },
            Default::default(),
        )
        .await;
    let error = outcome.result().as_ref().expect_err("retry must terminate");
    assert_eq!(error.kind(), CasErrorKind::FlowTimeout);
    assert_ne!(
        outcome.report().outcome(),
        CasExecutionOutcome::ErrorTotalBudgetExceeded
    );
}

#[test]
fn test_random_delay_validation_reports_the_invalid_field() {
    let invalid = CasExecutor::<usize, ()>::builder()
        .random_delay(Duration::from_secs(1), Duration::ZERO)
        .build()
        .expect_err("reversed delay bounds");
    assert!(!invalid.message().is_empty());
    assert!(invalid.to_string().contains(invalid.field()));
    assert!(invalid.to_string().contains(invalid.message()));
    let executor = CasExecutor::<usize, ()>::builder()
        .random_delay(Duration::ZERO, Duration::ZERO)
        .build()
        .expect("zero delay is valid");
    let state = AtomicRef::from_value(0usize);
    assert_eq!(
        executor
            .execute_result(&state, |_: &usize| CasDecision::finish(3))
            .expect("finish")
            .output(),
        &3
    );
}
