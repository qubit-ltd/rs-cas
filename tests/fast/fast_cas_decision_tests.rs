// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::FastCasDecision;

#[test]
fn test_fast_cas_decision_constructors_create_variants() {
    assert_eq!(
        FastCasDecision::<_, &'static str>::update(2, "ok"),
        FastCasDecision::Update {
            next: 2,
            output: "ok"
        }
    );
    assert_eq!(
        FastCasDecision::<_, &'static str>::finish("done"),
        FastCasDecision::Finish { output: "done" }
    );
    assert_eq!(
        FastCasDecision::<usize, _>::abort("error"),
        FastCasDecision::Abort { error: "error" }
    );
}
