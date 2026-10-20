// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasAlert;
use qubit_cas::event::CasAlertHook;
use qubit_function::ArcConsumer;

fn accept_alert_hook(_hook: CasAlertHook) {}

#[test]
fn test_alert_hook_alias_accepts_arc_consumer() {
    let hook: CasAlertHook = ArcConsumer::new(|_alert: &CasAlert| {});
    accept_alert_hook(hook);
}
