/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

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
    #[inline]
    pub fn mode(&self) -> CasObservabilityMode {
        self.mode
    }

    /// Sets the selected observability mode.
    ///
    /// # Parameters
    /// - `mode`: New observability mode to use.
    ///
    /// # Returns
    /// Updated builder-style config (consumes self).
    #[inline]
    pub fn with_mode(mut self, mode: CasObservabilityMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns the listener panic policy.
    ///
    /// # Returns
    /// Current [`ListenerPanicPolicy`] for event/alert hooks.
    #[inline]
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
    #[inline]
    pub fn with_listener_panic_policy(mut self, policy: ListenerPanicPolicy) -> Self {
        self.listener_panic_policy = policy;
        self
    }

    /// Returns configured contention thresholds, when alerting is enabled.
    ///
    /// # Returns
    /// `Some(thresholds)` if alert mode is active, otherwise `None`.
    #[inline]
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
    #[inline]
    pub fn with_contention_thresholds(mut self, thresholds: ContentionThresholds) -> Self {
        self.mode = CasObservabilityMode::EventStreamWithAlert;
        self.contention_thresholds = Some(thresholds);
        self
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
