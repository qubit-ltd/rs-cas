// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasStrategy;

#[test]
fn test_default_strategy_selects_latency_first() {
    assert_eq!(CasStrategy::default(), CasStrategy::LatencyFirst);
}
