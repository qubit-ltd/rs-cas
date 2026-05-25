/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_atomic::AtomicRef;
use qubit_cas::{
    CasDecision,
    CasExecutor,
};

use crate::support::TestError;

/// Verifies success accessors for updated and finished outcomes.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_success_accessors_cover_updated_and_finished_variants() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let updated = executor
        .execute(&state, |current: &usize| CasDecision::update(*current + 1, "updated"))
        .expect("update should succeed");
    assert!(updated.previous().is_some());
    assert_eq!(updated.clone().into_output(), "updated");

    let finished = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, &'static str, TestError>::finish("finished")
        })
        .expect("finish should succeed");
    assert!(finished.previous().is_none());
    assert_eq!(finished.into_output(), "finished");
}
