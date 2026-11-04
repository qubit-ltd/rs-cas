// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for CAS decision construction and snapshot-sharing clones.

use std::sync::Arc;

use qubit_cas::CasDecision;

/// Proves that cloning a decision shares its snapshot without requiring
/// `Clone`.
#[derive(Debug, PartialEq, Eq)]
struct NonCloneSnapshot;

#[test]
fn test_decision_clone_preserves_variants_without_cloning_snapshot() {
    let next = Arc::new(NonCloneSnapshot);
    let decisions: [CasDecision<NonCloneSnapshot, String, String>; 4] = [
        CasDecision::update_arc(Arc::clone(&next), "updated".to_owned()),
        CasDecision::finish("finished".to_owned()),
        CasDecision::retry("retry".to_owned()),
        CasDecision::abort("abort".to_owned()),
    ];
    for decision in decisions {
        let cloned = decision.clone();
        assert_eq!(
            cloned, decision,
            "cloning must preserve the decision variant and payload"
        );
        if let CasDecision::Update { next: cloned_next, .. } = cloned {
            assert!(
                Arc::ptr_eq(&cloned_next, &next),
                "cloning an update must share its snapshot allocation"
            );
        }
    }
}

#[test]
fn test_decision_constructors_create_expected_variants() {
    let next = Arc::new(7usize);
    let update = CasDecision::<usize, &'static str, &'static str>::update_arc(Arc::clone(&next), "ok");
    match update {
        CasDecision::Update {
            next: actual_next,
            output,
        } => {
            assert!(
                Arc::ptr_eq(&actual_next, &next),
                "update_arc must retain the supplied snapshot allocation"
            );
            assert_eq!(output, "ok", "update_arc must retain its business output");
        }
        other => panic!("expected update decision, got {other:?}"),
    }

    let finish = CasDecision::<usize, &'static str, &'static str>::finish("done");
    assert_eq!(
        finish,
        CasDecision::Finish { output: "done" },
        "finish must construct a successful decision without an update"
    );

    let retry = CasDecision::<usize, &'static str, &'static str>::retry("retry");
    assert_eq!(
        retry,
        CasDecision::Retry("retry"),
        "retry must retain the retryable business failure"
    );

    let abort = CasDecision::<usize, &'static str, &'static str>::abort("abort");
    assert_eq!(
        abort,
        CasDecision::Abort("abort"),
        "abort must retain the terminal business failure"
    );
}
