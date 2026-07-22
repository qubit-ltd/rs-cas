// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contention alert emitted from an observed CAS execution.

use crate::report::CasExecutionReport;

use super::ContentionThresholds;

/// Alert emitted when an execution crosses configured contention thresholds.
#[derive(Debug, Clone)]
pub struct CasAlert {
    /// Report that triggered the alert.
    report: CasExecutionReport,
    /// Thresholds that were crossed.
    thresholds: ContentionThresholds,
}

impl CasAlert {
    /// Creates a contention alert.
    ///
    /// # Parameters
    /// - `report`: The execution report that crossed thresholds.
    /// - `thresholds`: The thresholds that were exceeded.
    ///
    /// # Returns
    /// A new [`CasAlert`] value.
    #[inline]
    pub(crate) fn contention(
        report: CasExecutionReport,
        thresholds: ContentionThresholds,
    ) -> Self {
        Self { report, thresholds }
    }

    /// Returns the report that triggered this alert.
    ///
    /// # Returns
    /// Reference to the [`CasExecutionReport`] that caused the alert.
    #[inline(always)]
    pub fn report(&self) -> &CasExecutionReport {
        &self.report
    }

    /// Returns the thresholds that were crossed.
    ///
    /// # Returns
    /// The [`ContentionThresholds`] used for this alert.
    #[inline(always)]
    pub fn thresholds(&self) -> ContentionThresholds {
        self.thresholds
    }
}
