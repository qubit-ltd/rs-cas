/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

use crate::support::NonCloneValue;

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
    let executor = CasExecutor::<usize>::builder()
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
    assert_eq!(success.context().attempt(), 1);
    assert_eq!(success.context().max_attempts(), 3);
    assert_eq!(success.context().max_retries(), 2);
    assert_eq!(success.context().max_operation_elapsed(), None);
    assert_eq!(success.context().max_total_elapsed(), None);
    assert!(success.context().total_elapsed() >= success.context().attempt_elapsed());
    assert_eq!(success.context().attempt_timeout(), None);
    assert_eq!(success.context().next_delay(), None);
}
