/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_cas::ListenerPanicPolicy;

/// Verifies listener panic policy default is `Propagate`.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_listener_panic_policy_default_is_propagate() {
    assert_eq!(
        ListenerPanicPolicy::default(),
        ListenerPanicPolicy::Propagate
    );
}
