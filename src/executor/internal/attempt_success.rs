// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Successful result produced by one retry attempt.

use std::sync::Arc;

/// Success payload produced before retry-context enrichment.
pub(in crate::executor) enum AttemptSuccess<T, R> {
    /// One compare-and-swap write succeeded.
    Updated {
        /// State observed by the successful attempt.
        previous: Arc<T>,
        /// State installed by the successful attempt.
        current: Arc<T>,
        /// Business output returned by the operation.
        output: R,
    },
    /// The operation completed successfully without writing.
    Finished {
        /// State observed by the successful attempt.
        current: Arc<T>,
        /// Business output returned by the operation.
        output: R,
    },
}
