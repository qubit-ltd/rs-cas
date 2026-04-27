/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::sync::Arc;

use qubit_cas::CasDecision;

use crate::support::TestError;

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
    let update = CasDecision::<usize, &'static str, TestError>::update_arc(Arc::clone(&next), "ok");
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

    let finish = CasDecision::<usize, &'static str, TestError>::finish("done");
    assert_eq!(finish, CasDecision::Finish { output: "done" });

    let retry = CasDecision::<usize, &'static str, TestError>::retry(TestError("retry"));
    assert_eq!(retry, CasDecision::Retry(TestError("retry")));

    let abort = CasDecision::<usize, &'static str, TestError>::abort(TestError("abort"));
    assert_eq!(abort, CasDecision::Abort(TestError("abort")));
}
