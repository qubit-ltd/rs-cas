// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS lifecycle event payload.

use std::time::Instant;

use super::CasContext;
use crate::error::CasAttemptFailureKind;
use crate::report::CasExecutionReport;

/// Lifecycle event emitted by a CAS execution.
#[derive(Debug, Clone)]
pub enum CasEvent {
    /// The execution started before the first attempt.
    ExecutionStarted {
        /// Instant captured when the execution started.
        started_at: Instant,
    },

    /// One attempt failed.
    AttemptFailed {
        /// Context captured for the failed attempt.
        context: CasContext,
        /// Attempt-level failure kind.
        kind: CasAttemptFailureKind,
    },

    /// The CAS rule requested a retry after a failed operation.
    /// This is an intent event, including on the final allowed attempt. Budget
    /// checks may reject the request; use the terminal report's attempts_total
    /// to count admitted operations rather than counting this event.
    RetryRequested {
        /// Context captured after the failed attempt.
        context: CasContext,
    },

    /// The execution finished and produced a report.
    ExecutionFinished {
        /// Final execution report.
        report: CasExecutionReport,
    },
}
