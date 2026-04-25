/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use qubit_cas::CasTimeoutPolicy;

/// Verifies the default timeout policy retries timed-out attempts.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_timeout_policy_default_is_retry() {
    assert_eq!(CasTimeoutPolicy::default(), CasTimeoutPolicy::Retry);
}
