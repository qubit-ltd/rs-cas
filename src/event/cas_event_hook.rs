// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared hook type for CAS lifecycle events.

use qubit_function::ArcConsumer;

use super::CasEvent;

/// Shared hook invoked for CAS lifecycle events.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicUsize;
/// use std::sync::atomic::Ordering;
///
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasEvent;
/// use qubit_cas::event::CasEventHook;
/// use qubit_function::ArcConsumer;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasHooks;
///
/// let finished = Arc::new(AtomicUsize::new(0));
/// let observed = Arc::clone(&finished);
/// let callback: CasEventHook = ArcConsumer::new(move |event: &CasEvent| {
///     if let CasEvent::ExecutionFinished { report } = event {
///         observed.store(report.attempts_total() as usize, Ordering::SeqCst);
///     }
/// });
/// let hooks = CasHooks::new().on_event(callback);
/// let state = AtomicRef::from_value(3usize);
/// let outcome = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute_with_hooks(&state, |_: &usize| CasDecision::finish(()), hooks);
/// assert!(outcome.is_ok());
/// assert_eq!(finished.load(Ordering::SeqCst), 1);
/// ```
pub type CasEventHook = ArcConsumer<CasEvent>;
