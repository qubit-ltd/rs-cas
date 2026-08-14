// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;

use crate::support::TestError;

/// Verifies retry-on-timeout continues with a later attempt.
///
/// # Returns
/// This test returns nothing.
#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_retry_on_timeout_continues_execution() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .attempt_timeout(Some(Duration::from_millis(10)))
        .retry_on_timeout()
        .build()
        .expect("retry-on-timeout executor should build");

    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let success = executor
        .execute_async(&state, {
            let attempts = Arc::clone(&attempts);
            move |_current: Arc<usize>| {
                let attempt =
                    attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move {
                    if attempt == 0 {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                    CasDecision::<usize, (), TestError>::finish(())
                }
            }
        })
        .await
        .expect("retry-on-timeout should allow the second attempt");

    assert_eq!(success.attempts(), 2);
}

/// Verifies abort-on-timeout terminates the execution after the first timeout.
///
/// # Returns
/// This test returns nothing.
#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_abort_on_timeout_terminates_execution() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .attempt_timeout(Some(Duration::from_millis(10)))
        .abort_on_timeout()
        .build()
        .expect("abort-on-timeout executor should build");

    let error = executor
        .execute_async(&state, |_current: Arc<usize>| async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            CasDecision::<usize, (), TestError>::finish(())
        })
        .await
        .expect_err("abort-on-timeout should return an error");

    assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
    assert_eq!(error.attempts(), 1);
}
