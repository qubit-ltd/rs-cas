// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasAttemptFailureKind;

/// Returns a stable assertion label for a failure kind.
fn label_of(kind: CasAttemptFailureKind) -> &'static str {
    match kind {
        CasAttemptFailureKind::Conflict => "conflict",
        CasAttemptFailureKind::Retry => "retry",
        CasAttemptFailureKind::Abort => "abort",
        CasAttemptFailureKind::Timeout => "timeout",
    }
}

/// Verifies that each failure kind has a distinct stable label.
#[test]
fn test_failure_kind_labels_cover_all_variants() {
    assert_eq!(label_of(CasAttemptFailureKind::Conflict), "conflict");
    assert_eq!(label_of(CasAttemptFailureKind::Retry), "retry");
    assert_eq!(label_of(CasAttemptFailureKind::Abort), "abort");
    assert_eq!(label_of(CasAttemptFailureKind::Timeout), "timeout");
}
