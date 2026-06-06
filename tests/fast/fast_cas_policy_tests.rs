// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::FastCasPolicy;

#[test]
fn test_fast_cas_policy_once_uses_one_attempt() {
    let policy = FastCasPolicy::once();

    assert_eq!(policy, FastCasPolicy::Once);
    assert_eq!(policy.max_attempts(), 1);
    assert!(!policy.should_yield_before(2));
}

#[test]
fn test_fast_cas_policy_spin_normalizes_attempts() {
    let policy = FastCasPolicy::spin(0);

    assert_eq!(policy, FastCasPolicy::Spin { max_attempts: 1 });
    assert_eq!(policy.max_attempts(), 1);
    assert!(!policy.should_yield_before(2));
}

#[test]
fn test_fast_cas_policy_spin_yield_caps_spin_attempts() {
    let policy = FastCasPolicy::spin_yield(10, 3);

    assert_eq!(
        policy,
        FastCasPolicy::SpinYield {
            spin_attempts: 3,
            max_attempts: 3,
        }
    );
    assert_eq!(policy.max_attempts(), 3);
    assert!(!policy.should_yield_before(3));
    assert!(policy.should_yield_before(4));
}
