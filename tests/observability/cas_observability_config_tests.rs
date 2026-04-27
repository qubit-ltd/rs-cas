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

/// Verifies mode switches clear thresholds when alerts are disabled.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_observability_config_mode_switches_clear_alert_thresholds() {
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let config = CasObservabilityConfig::event_stream_with_alert(thresholds)
        .with_listener_panic_policy(ListenerPanicPolicy::Isolate);

    let event_stream = config.clone().with_event_stream();
    assert_eq!(event_stream.mode(), CasObservabilityMode::EventStream);
    assert_eq!(
        event_stream.listener_panic_policy(),
        ListenerPanicPolicy::Isolate
    );
    assert!(event_stream.contention_thresholds().is_none());

    let report_only = config.with_report_only();
    assert_eq!(report_only.mode(), CasObservabilityMode::ReportOnly);
    assert_eq!(
        report_only.listener_panic_policy(),
        ListenerPanicPolicy::Isolate
    );
    assert!(report_only.contention_thresholds().is_none());
}

/// Verifies alert switches require and preserve explicit thresholds.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_observability_config_alert_switches_install_thresholds() {
    let first_thresholds = ContentionThresholds::new(2, 1, 0.5);
    let second_thresholds = ContentionThresholds::new(4, 2, 0.75);

    let config = CasObservabilityConfig::report_only()
        .with_event_stream_with_alert(first_thresholds)
        .without_contention_alerts()
        .with_contention_thresholds(second_thresholds);

    assert_eq!(config.mode(), CasObservabilityMode::EventStreamWithAlert);
    assert_eq!(config.contention_thresholds(), Some(second_thresholds));
}
