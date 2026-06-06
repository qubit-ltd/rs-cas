// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Non-cloneable value used by CAS tests.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NonCloneValue {
    pub(crate) value: &'static str,
}
