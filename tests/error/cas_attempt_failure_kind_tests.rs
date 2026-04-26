/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use qubit_cas::CasAttemptFailureKind;

/// Converts each failure kind to a stable label.
///
/// # Parameters
/// - `kind`: Failure kind to label.
///
/// # Returns
/// Stable string label for assertions.
fn label_of(kind: CasAttemptFailureKind) -> &'static str {
    match kind {
        CasAttemptFailureKind::Conflict => "conflict",
        CasAttemptFailureKind::Retry => "retry",
        CasAttemptFailureKind::Abort => "abort",
        CasAttemptFailureKind::Timeout => "timeout",
    }
}

/// Verifies all failure kinds remain addressable and distinct.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_failure_kind_labels_cover_all_variants() {
    assert_eq!(label_of(CasAttemptFailureKind::Conflict), "conflict");
    assert_eq!(label_of(CasAttemptFailureKind::Retry), "retry");
    assert_eq!(label_of(CasAttemptFailureKind::Abort), "abort");
    assert_eq!(label_of(CasAttemptFailureKind::Timeout), "timeout");
}
