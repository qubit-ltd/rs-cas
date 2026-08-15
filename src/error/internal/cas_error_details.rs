// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compact heap-owned details retained by terminal CAS errors.

use crate::error::CasRetryFailure;
use crate::event::CasContext;

/// Heap-stored retry details that keep `CasError` compact in `Result` values.
#[derive(Clone)]
pub(in crate::error) struct CasErrorDetails {
    /// Structured terminal failure selected by the retry layer.
    pub(in crate::error) failure: CasRetryFailure,

    /// Copied CAS context captured when execution stopped.
    pub(in crate::error) context: CasContext,
}
