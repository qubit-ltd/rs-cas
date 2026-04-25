/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::sync::Arc;

use qubit_cas::CasAttemptFailure;

use crate::support::TestError;

/// Verifies attempt failure accessors classify variants correctly.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_attempt_failure_accessors_work() {
    let state = Arc::new(7usize);
    let conflict = CasAttemptFailure::<usize, TestError>::Conflict {
        current: Arc::clone(&state),
    };
    assert!(conflict.is_conflict());
    assert!(!conflict.is_retry());
    assert_eq!(conflict.error(), None);
    assert_eq!(**conflict.current(), 7);

    let retry = CasAttemptFailure::Retry {
        current: Arc::clone(&state),
        error: TestError("retry"),
    };
    assert!(retry.is_retry());
    assert_eq!(retry.error(), Some(&TestError("retry")));

    let abort = CasAttemptFailure::Abort {
        current: Arc::clone(&state),
        error: TestError("abort"),
    };
    assert!(abort.is_abort());
    assert_eq!(abort.error(), Some(&TestError("abort")));

    let timeout = CasAttemptFailure::<usize, TestError>::Timeout { current: state };
    assert!(timeout.is_timeout());
    assert_eq!(timeout.error(), None);
    assert!(timeout.to_string().contains("timed out"));
}
