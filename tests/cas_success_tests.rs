// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public success-result contracts for snapshot sharing and variant accessors.

use std::sync::Arc;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

#[derive(Debug)]
struct NonCloneSnapshot;

#[test]
fn test_success_clone_preserves_non_clone_snapshot_identity() {
    let state = AtomicRef::from_value(NonCloneSnapshot);
    let executor = CasExecutor::<NonCloneSnapshot, String>::builder()
        .build()
        .expect("valid builder");
    let updated = executor
        .execute_result(&state, |_: &NonCloneSnapshot| {
            CasDecision::update(NonCloneSnapshot, "updated".to_owned())
        })
        .expect("update succeeds");
    let finished = executor
        .execute_result(&state, |_: &NonCloneSnapshot| {
            CasDecision::finish("finished".to_owned())
        })
        .expect("finish succeeds");
    for success in [updated, finished] {
        let cloned = success.clone();
        assert_eq!(
            cloned.is_updated(),
            success.is_updated(),
            "clone preserves the success variant"
        );
        assert!(
            Arc::ptr_eq(cloned.current(), success.current()),
            "clone shares the current snapshot"
        );
        if let Some(previous) = success.previous() {
            assert!(
                Arc::ptr_eq(cloned.previous().expect("updated retains previous"), previous),
                "clone shares the previous snapshot"
            );
        } else {
            assert!(cloned.previous().is_none(), "finished clone has no previous snapshot");
        }
        assert_eq!(cloned.output(), success.output(), "clone preserves business output");
        assert_eq!(cloned.context(), success.context(), "clone preserves retry context");
    }
}

#[test]
fn test_success_accessors_cover_updated_and_finished_variants() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, &'static str>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let updated = executor
        .execute(&state, |current: &usize| CasDecision::update(*current + 1, "updated"))
        .expect("update should succeed");
    assert!(
        updated.previous().is_some(),
        "updated success retains a previous snapshot"
    );
    assert_eq!(
        updated.clone().into_output(),
        "updated",
        "updated success yields its business output"
    );

    let finished = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, &'static str, &'static str>::finish("finished")
        })
        .expect("finish should succeed");
    assert!(
        finished.previous().is_none(),
        "finished success has no previous snapshot"
    );
    assert_eq!(
        finished.into_output(),
        "finished",
        "finished success yields its business output"
    );
}
