// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stable categories for retry diagnostics exposed by CAS.

/// Category of an execution diagnostic, independent of retry implementation
/// types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CasDiagnosticKind {
    /// Cancellation was observed by the retry infrastructure.
    Cancellation,
    /// A retry control or completion callback failed.
    Callback,
    /// The monotonic clock could not provide a valid reading.
    Clock,
    /// A retry timer failed.
    Timer,
    /// Another retry infrastructure component failed.
    Infrastructure,
    /// A newer retry version supplied an unrecognized terminal reason.
    UnknownRetryReason,
}
