// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Observability configuration shared by CAS executions.

use super::{CasObservabilityMode, ContentionThresholds, ListenerPanicPolicy};

/// Observability settings shared by every execution of an executor.
#[derive(Debug, Clone, PartialEq)]
pub struct CasObservabilityConfig {
    /// Selected observability mode.
    mode: CasObservabilityMode,
    /// Panic policy for event and alert listeners.
    listener_panic_policy: ListenerPanicPolicy,
    /// Optional contention threshold used for alerting.
    contention_thresholds: Option<ContentionThresholds>,
}

impl CasObservabilityConfig {
    /// Creates a report-only observability configuration (lowest overhead).
    ///
    /// Equivalent to the default.
    ///
    /// # Returns
    /// A [`CasObservabilityConfig`] with report-only mode and no alerts.
    #[inline]
    pub fn report_only() -> Self {
        Self::default()
    }

    /// Creates an event-stream observability configuration.
    ///
    /// # Returns
    /// A [`CasObservabilityConfig`] that emits lifecycle events but no alerts.
    #[inline]
    pub fn event_stream() -> Self {
        Self {
            mode: CasObservabilityMode::EventStream,
            ..Self::default()
        }
    }

    /// Creates an event-stream configuration with contention alerts.
    ///
    /// # Parameters
    /// - `thresholds`: Thresholds used to detect hot contention for alerts.
    ///
    /// # Returns
    /// A [`CasObservabilityConfig`] with event streaming and alert enabled.
    #[inline]
    pub fn event_stream_with_alert(thresholds: ContentionThresholds) -> Self {
        Self {
            mode: CasObservabilityMode::EventStreamWithAlert,
            contention_thresholds: Some(thresholds),
            ..Self::default()
        }
    }

    /// Returns the selected observability mode.
    ///
    /// # Returns
    /// The current [`CasObservabilityMode`].
    #[inline(always)]
    pub fn mode(&self) -> CasObservabilityMode {
        self.mode
    }

    /// Switches to report-only observability.
    ///
    /// This disables lifecycle event streaming and clears contention
    /// thresholds.
    ///
    /// # Returns
    /// Updated builder-style config (consumes self).
    #[inline(always)]
    pub fn with_report_only(mut self) -> Self {
        self.mode = CasObservabilityMode::ReportOnly;
        self.contention_thresholds = None;
        self
    }

    /// Switches to event-stream observability without alerts.
    ///
    /// This enables lifecycle events and clears contention thresholds.
    ///
    /// # Returns
    /// Updated builder-style config (consumes self).
    #[inline(always)]
    pub fn with_event_stream(mut self) -> Self {
        self.mode = CasObservabilityMode::EventStream;
        self.contention_thresholds = None;
        self
    }

    /// Switches to event-stream observability with contention alerts.
    ///
    /// # Parameters
    /// - `thresholds`: Thresholds used to detect hot contention for alerts.
    ///
    /// # Returns
    /// Updated builder-style config with alert mode enabled (consumes self).
    #[inline(always)]
    pub fn with_event_stream_with_alert(mut self, thresholds: ContentionThresholds) -> Self {
        self.mode = CasObservabilityMode::EventStreamWithAlert;
        self.contention_thresholds = Some(thresholds);
        self
    }

    /// Disables contention alerts while keeping lifecycle event streaming.
    ///
    /// # Returns
    /// Updated builder-style config with event streaming and no alert
    /// thresholds.
    #[inline(always)]
    pub fn without_contention_alerts(self) -> Self {
        self.with_event_stream()
    }

    /// Returns the listener panic policy.
    ///
    /// # Returns
    /// Current [`ListenerPanicPolicy`] for event/alert hooks.
    #[inline(always)]
    pub fn listener_panic_policy(&self) -> ListenerPanicPolicy {
        self.listener_panic_policy
    }

    /// Sets the listener panic policy.
    ///
    /// # Parameters
    /// - `policy`: How to handle panics from registered hooks.
    ///
    /// # Returns
    /// Updated builder-style config (consumes self).
    #[inline(always)]
    pub fn with_listener_panic_policy(mut self, policy: ListenerPanicPolicy) -> Self {
        self.listener_panic_policy = policy;
        self
    }

    /// Returns configured contention thresholds, when alerting is enabled.
    ///
    /// # Returns
    /// `Some(thresholds)` if alert mode is active, otherwise `None`.
    #[inline(always)]
    pub fn contention_thresholds(&self) -> Option<ContentionThresholds> {
        self.contention_thresholds
    }

    /// Sets contention thresholds and enables alert-capable event streaming.
    ///
    /// # Parameters
    /// - `thresholds`: Thresholds for classifying executions as hot contention.
    ///
    /// # Returns
    /// Updated builder-style config with alert mode enabled (consumes self).
    #[inline(always)]
    pub fn with_contention_thresholds(self, thresholds: ContentionThresholds) -> Self {
        self.with_event_stream_with_alert(thresholds)
    }
}

impl Default for CasObservabilityConfig {
    /// Returns report-only observability (the default, lowest overhead mode).
    ///
    /// # Returns
    /// Config with `ReportOnly` mode, propagate panics, and no contention
    /// thresholds.
    #[inline]
    fn default() -> Self {
        Self {
            mode: CasObservabilityMode::ReportOnly,
            listener_panic_policy: ListenerPanicPolicy::Propagate,
            contention_thresholds: None,
        }
    }
}
