/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_cas::CasErrorKind;

#[test]
fn test_cas_error_kind_debug_clone_copy_and_equality() {
    let kinds = [
        CasErrorKind::Abort,
        CasErrorKind::Conflict,
        CasErrorKind::RetryExhausted,
        CasErrorKind::AttemptTimeout,
        CasErrorKind::MaxOperationElapsedExceeded,
        CasErrorKind::MaxTotalElapsedExceeded,
    ];

    for kind in kinds {
        let copied = kind;
        let cloned = kind;
        assert_eq!(copied, cloned);
        assert!(!format!("{kind:?}").is_empty());
    }
}
