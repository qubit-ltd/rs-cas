// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================
//! Lifecycle event and alert dispatch boundaries.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::event::{CasAlertHook, CasEvent, CasEventHook};
use crate::observability::{
    CasAlert, CasObservabilityConfig, CasObservabilityMode, ListenerPanicPolicy,
};
use qubit_function::Consumer;

/// Returns whether an event should be built and dispatched.
pub(super) fn should_emit_events(
    observability: &CasObservabilityConfig,
    hook: &Option<CasEventHook>,
) -> bool {
    observability.mode() != CasObservabilityMode::ReportOnly && hook.is_some()
}

/// Dispatches one event according to the configured listener panic policy.
pub(super) fn dispatch_event(
    observability: &CasObservabilityConfig,
    hook: &CasEventHook,
    event: CasEvent,
) {
    match observability.listener_panic_policy() {
        ListenerPanicPolicy::Propagate => hook.accept(&event),
        ListenerPanicPolicy::Isolate => {
            let _ = catch_unwind(AssertUnwindSafe(|| hook.accept(&event)));
        }
    }
}

/// Dispatches one optional alert according to the configured panic policy.
pub(super) fn dispatch_alert(
    observability: &CasObservabilityConfig,
    hook: &Option<CasAlertHook>,
    alert: CasAlert,
) {
    if let Some(hook) = hook {
        match observability.listener_panic_policy() {
            ListenerPanicPolicy::Propagate => hook.accept(&alert),
            ListenerPanicPolicy::Isolate => {
                let _ = catch_unwind(AssertUnwindSafe(|| hook.accept(&alert)));
            }
        }
    }
}
