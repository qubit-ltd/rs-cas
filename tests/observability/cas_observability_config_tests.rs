/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use qubit_cas::{
    CasObservabilityConfig, CasObservabilityMode, ContentionThresholds, ListenerPanicPolicy,
};

/// Verifies report-only helper matches the default configuration.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_observability_config_report_only_equals_default() {
    assert_eq!(
        CasObservabilityConfig::report_only(),
        CasObservabilityConfig::default()
    );
}

/// Verifies event-stream helpers and builder mutators.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_observability_config_builders_work() {
    let thresholds = ContentionThresholds::new(4, 2, 0.75);
    let config = CasObservabilityConfig::event_stream()
        .with_mode(CasObservabilityMode::EventStreamWithAlert)
        .with_listener_panic_policy(ListenerPanicPolicy::Isolate)
        .with_contention_thresholds(thresholds);

    assert_eq!(config.mode(), CasObservabilityMode::EventStreamWithAlert);
    assert_eq!(config.listener_panic_policy(), ListenerPanicPolicy::Isolate);
    assert_eq!(config.contention_thresholds(), Some(thresholds));
}

/// Verifies `event_stream_with_alert` installs alert thresholds.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_observability_config_event_stream_with_alert() {
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let config = CasObservabilityConfig::event_stream_with_alert(thresholds);

    assert_eq!(config.mode(), CasObservabilityMode::EventStreamWithAlert);
    assert_eq!(config.contention_thresholds(), Some(thresholds));
}
