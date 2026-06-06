// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasObservabilityMode;

/// Verifies observability mode default is report-only.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_observability_mode_default_is_report_only() {
    assert_eq!(
        CasObservabilityMode::default(),
        CasObservabilityMode::ReportOnly
    );
}
