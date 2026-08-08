// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;

use qubit_cas::CasDecision;

/// Verifies all CAS decision constructors create the expected variants.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_decision_constructors_create_expected_variants() {
    let next = Arc::new(7usize);
    let update =
        CasDecision::<usize, &'static str, &'static str>::update_arc(Arc::clone(&next), "ok");
    match update {
        CasDecision::Update {
            next: actual_next,
            output,
        } => {
            assert!(Arc::ptr_eq(&actual_next, &next));
            assert_eq!(output, "ok");
        }
        other => panic!("expected update decision, got {other:?}"),
    }

    let finish = CasDecision::<usize, &'static str, &'static str>::finish("done");
    assert_eq!(finish, CasDecision::Finish { output: "done" });

    let retry = CasDecision::<usize, &'static str, &'static str>::retry("retry");
    assert_eq!(retry, CasDecision::Retry("retry"));

    let abort = CasDecision::<usize, &'static str, &'static str>::abort("abort");
    assert_eq!(abort, CasDecision::Abort("abort"));
}
