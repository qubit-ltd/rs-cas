// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;

use qubit_cas::CasAttemptFailure;
use qubit_cas::CasAttemptFailureKind;

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
    assert!(!conflict.is_abort());
    assert!(!conflict.is_timeout());
    assert_eq!(conflict.kind(), CasAttemptFailureKind::Conflict);
    assert_eq!(conflict.error(), None);
    assert_eq!(**conflict.current(), 7);
    assert_eq!(conflict.to_string(), "compare-and-swap conflict");

    let retry = CasAttemptFailure::Retry {
        current: Arc::clone(&state),
        error: TestError("retry"),
    };
    assert!(!retry.is_conflict());
    assert!(retry.is_retry());
    assert!(!retry.is_abort());
    assert!(!retry.is_timeout());
    assert_eq!(retry.kind(), CasAttemptFailureKind::Retry);
    assert_eq!(retry.error(), Some(&TestError("retry")));

    let abort = CasAttemptFailure::Abort {
        current: Arc::clone(&state),
        error: TestError("abort"),
    };
    assert!(!abort.is_conflict());
    assert!(!abort.is_retry());
    assert!(abort.is_abort());
    assert!(!abort.is_timeout());
    assert_eq!(abort.kind(), CasAttemptFailureKind::Abort);
    assert_eq!(abort.error(), Some(&TestError("abort")));
    assert_eq!(abort.to_string(), "aborted CAS failure: abort");

    let timeout = CasAttemptFailure::<usize, TestError>::Timeout { current: state };
    assert!(!timeout.is_conflict());
    assert!(!timeout.is_retry());
    assert!(!timeout.is_abort());
    assert!(timeout.is_timeout());
    assert_eq!(timeout.kind(), CasAttemptFailureKind::Timeout);
    assert_eq!(timeout.error(), None);
    assert_eq!(**timeout.current(), 7);
    assert!(timeout.to_string().contains("timed out"));
}
