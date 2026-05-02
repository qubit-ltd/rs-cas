/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_cas::ContentionThresholds;

/// Verifies contention threshold constructor clamps ratio values.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_contention_thresholds_new_clamps_ratio() {
    let high = ContentionThresholds::new(3, 1, 1.8);
    let low = ContentionThresholds::new(3, 1, -0.2);

    assert_eq!(high.conflict_ratio(), 1.0);
    assert_eq!(low.conflict_ratio(), 0.0);
}

/// Verifies contention threshold default values.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_contention_thresholds_default_values() {
    let thresholds = ContentionThresholds::default();
    assert_eq!(thresholds.min_attempts(), 3);
    assert_eq!(thresholds.min_conflicts(), 1);
    assert_eq!(thresholds.conflict_ratio(), 0.30);
}
