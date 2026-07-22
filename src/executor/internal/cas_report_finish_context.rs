// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Terminal retry context used to finish one CAS report.

use std::time::Duration;

use crate::report::CasExecutionOutcome;

/// Snapshot of retry limits plus the terminal execution outcome.
pub(in crate::executor) struct CasReportFinishContext {
    /// Total attempts executed by the retry flow.
    pub(in crate::executor) attempts_total: u32,
    /// Configured maximum number of attempts.
    pub(in crate::executor) max_attempts: u32,
    /// Configured cumulative operation elapsed-time budget.
    pub(in crate::executor) max_operation_elapsed: Option<Duration>,
    /// Configured total retry-flow elapsed-time budget.
    pub(in crate::executor) max_total_elapsed: Option<Duration>,
    /// Terminal outcome assigned to the report.
    pub(in crate::executor) outcome: CasExecutionOutcome,
}

impl CasReportFinishContext {
    /// Creates a terminal report context from retry-layer values.
    #[inline]
    pub(in crate::executor) fn new(
        attempts_total: u32,
        max_attempts: u32,
        max_operation_elapsed: Option<Duration>,
        max_total_elapsed: Option<Duration>,
        outcome: CasExecutionOutcome,
    ) -> Self {
        Self {
            attempts_total,
            max_attempts,
            max_operation_elapsed,
            max_total_elapsed,
            outcome,
        }
    }
}
