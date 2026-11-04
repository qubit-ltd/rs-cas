// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::ContentionThresholds;

/// Verifies that the constructor clamps conflict ratios to the supported range.
#[test]
fn test_contention_thresholds_new_clamps_ratio() {
    let high = ContentionThresholds::new(3, 1, 1.8);
    let low = ContentionThresholds::new(3, 1, -0.2);
    let nan = ContentionThresholds::new(3, 1, f64::NAN);

    assert_eq!(high.conflict_ratio(), 1.0, "ratios above one must clamp to one");
    assert_eq!(low.conflict_ratio(), 0.0, "negative ratios must clamp to zero");
    assert_eq!(nan.conflict_ratio(), 0.0, "NaN ratios must normalize to zero");
}

/// Verifies the default contention threshold values.
#[test]
fn test_contention_thresholds_default_values() {
    let thresholds = ContentionThresholds::default();
    assert_eq!(thresholds.min_attempts(), 3, "default minimum attempts must be three");
    assert_eq!(thresholds.min_conflicts(), 1, "default minimum conflicts must be one");
    assert_eq!(thresholds.conflict_ratio(), 0.30, "default conflict ratio must be 30%");
}
