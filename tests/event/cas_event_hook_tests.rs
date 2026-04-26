/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use qubit_cas::CasEvent;
use qubit_cas::event::CasEventHook;
use qubit_function::Consumer;

/// Accepts an event hook alias to validate public API typing.
///
/// # Parameters
/// - `hook`: Shared event hook.
///
/// # Returns
/// This function returns nothing.
fn accept_event_hook(_hook: CasEventHook) {}

/// Verifies the event hook alias accepts rs-function consumers.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_event_hook_alias_accepts_arc_consumer() {
    let observed = Arc::new(AtomicBool::new(false));
    let observed_flag = Arc::clone(&observed);
    let hook: CasEventHook = (move |event: &CasEvent| {
        if matches!(event, CasEvent::ExecutionStarted { .. }) {
            observed_flag.store(true, Ordering::SeqCst);
        }
    })
    .into_arc();
    accept_event_hook(hook.clone());

    hook.accept(&CasEvent::ExecutionStarted {
        started_at: Instant::now(),
    });
    assert!(observed.load(Ordering::SeqCst));
}
