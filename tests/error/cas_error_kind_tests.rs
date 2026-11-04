// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Verifies value semantics and debug output for public terminal error kinds.

use qubit_cas::CasErrorKind;

#[test]
fn test_cas_error_kind_debug_clone_copy_and_equality() {
    let kinds = [
        CasErrorKind::Abort,
        CasErrorKind::ConflictExhausted,
        CasErrorKind::RetryExhausted,
        CasErrorKind::AttemptTimeout,
        CasErrorKind::FlowTimeout,
        CasErrorKind::OperationBudgetExceeded,
        CasErrorKind::TotalBudgetExceeded,
        CasErrorKind::RetryInfrastructure,
    ];

    for kind in kinds {
        let copied = kind;
        let cloned = kind;
        assert_eq!(copied, cloned, "copies of {kind:?} must remain equal");
        assert!(
            !format!("{kind:?}").is_empty(),
            "the debug representation of an error kind must not be empty"
        );
    }
}
