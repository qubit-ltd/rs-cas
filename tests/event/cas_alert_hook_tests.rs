// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::event::CasAlertHook;
use qubit_function::ArcConsumer;

/// Accepts an alert hook alias to validate public API typing.
///
/// # Parameters
/// - `hook`: Shared alert hook.
///
/// # Returns
/// This function returns nothing.
fn accept_alert_hook(_hook: CasAlertHook) {}

/// Verifies the alert hook alias accepts rs-function consumers.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_alert_hook_alias_accepts_arc_consumer() {
    let hook: CasAlertHook =
        ArcConsumer::new(|_alert: &qubit_cas::CasAlert| {});
    accept_alert_hook(hook);
}
