// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private storage for terminal CAS diagnostics.

use crate::error::CasDiagnostic;
use crate::error::CasTermination;
use crate::event::CasContext;

/// Owns the terminal snapshot shared by the public CAS error projections.
///
/// Keeping these values together lets the error retain retry context and
/// diagnostics after the retry layer has finished, without storing its
/// generic business error type here.
#[derive(Debug, Clone)]
pub(crate) struct CasErrorDetails {
    /// Structured reason that stopped execution, independent of any retained
    /// error from the final business attempt.
    pub(crate) termination: CasTermination,
    /// Retry counters, limits, and elapsed-time values captured at termination.
    pub(crate) context: CasContext,
    /// Infrastructure cause associated with the terminal reason, when present.
    pub(crate) diagnostic: Option<CasDiagnostic>,
    /// Callback failures recorded after the terminal result was frozen, in
    /// callback dispatch order.
    pub(crate) completion_diagnostics: Box<[CasDiagnostic]>,
}
