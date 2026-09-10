// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared hook type for CAS alerts.

use qubit_function::ArcConsumer;

use crate::observability::CasAlert;

/// Shared hook invoked for CAS alerts.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicUsize;
/// use std::sync::atomic::Ordering;
///
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasAlert;
/// use qubit_cas::event::CasAlertHook;
/// use qubit_function::ArcConsumer;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasHooks;
/// use qubit_cas::ContentionThresholds;
///
/// let state = AtomicRef::from_value(3usize);
/// let count = Arc::new(AtomicUsize::new(0));
/// let observed = Arc::clone(&count);
/// let thresholds = ContentionThresholds::new(1, 1, 1.0);
/// let callback: CasAlertHook = ArcConsumer::new(move |alert: &CasAlert| {
///     assert_eq!(alert.thresholds(), thresholds);
///     assert_eq!(alert.report().conflicts(), 1);
///     observed.fetch_add(1, Ordering::SeqCst);
/// });
/// let hooks = CasHooks::new().on_contention_alert(thresholds, callback);
/// let outcome = CasExecutor::<usize, ()>::builder().max_attempts(1).build().unwrap()
///     .execute_with_hooks(&state, |current: &usize| {
///         // Simulate another writer; avoid external side effects in real retry closures.
///         state.store(Arc::new(*current + 1));
///         CasDecision::update(*current + 2, ())
///     }, hooks);
/// assert!(outcome.is_err());
/// assert_eq!(count.load(Ordering::SeqCst), 1);
/// ```
pub type CasAlertHook = ArcConsumer<CasAlert>;
