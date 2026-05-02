/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

/// Observability mode used by the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasObservabilityMode {
    /// Produce only the final execution report.
    ReportOnly,
    /// Emit lifecycle events to registered listeners.
    EventStream,
    /// Emit lifecycle events and contention alerts.
    EventStreamWithAlert,
}

impl Default for CasObservabilityMode {
    /// Returns the lowest-overhead observability mode.
    ///
    /// # Returns
    /// [`CasObservabilityMode::ReportOnly`] (no events or alerts).
    #[inline]
    fn default() -> Self {
        Self::ReportOnly
    }
}
