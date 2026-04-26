/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! CAS lifecycle event payload.

use std::time::Instant;

use crate::error::CasAttemptFailureKind;
use crate::report::CasExecutionReport;

use super::CasContext;

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

    /// A retry has been requested by the CAS layer.
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
