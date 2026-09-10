// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS event and hook registrations.

use qubit_function::ArcConsumer;
use qubit_function::Consumer;

use super::CasAlertHook;
use super::CasEvent;
use super::CasEventHook;
use crate::observability::CasAlert;
use crate::observability::ContentionThresholds;

/// Per-execution hooks for observing CAS lifecycle events.
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
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasHooks;
///
/// let finished = Arc::new(AtomicUsize::new(0));
/// let observed = Arc::clone(&finished);
/// let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
///     if let CasEvent::ExecutionFinished { report } = event {
///         observed.store(report.attempts_total() as usize, Ordering::SeqCst);
///     }
/// });
/// let state = AtomicRef::from_value(3usize);
/// let outcome = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute_with_hooks(&state, |_: &usize| CasDecision::finish(()), hooks);
/// assert!(outcome.is_ok());
/// assert_eq!(finished.load(Ordering::SeqCst), 1);
/// ```
///
/// Alert registration requires explicit thresholds:
///
/// ```compile_fail
/// use qubit_cas::CasAlert;
/// use qubit_cas::CasHooks;
///
/// let _ = CasHooks::new().on_alert(|_: &CasAlert| {});
/// ```
#[derive(Clone)]
pub struct CasHooks {
    /// Hook invoked for lifecycle events.
    on_event: Option<CasEventHook>,
    /// Hook invoked when configured alert thresholds are crossed.
    on_alert: Option<CasAlertHook>,
    /// Optional thresholds used to trigger contention alerts.
    contention_thresholds: Option<ContentionThresholds>,
}

impl Default for CasHooks {
    /// Creates an empty hook set.
    ///
    /// # Returns
    /// A [`CasHooks`] value with every hook unset.
    #[inline]
    fn default() -> Self {
        Self {
            on_event: None,
            on_alert: None,
            contention_thresholds: None,
        }
    }
}

impl CasHooks {
    /// Creates an empty hook set.
    ///
    /// # Returns
    /// A [`CasHooks`] value with every hook unset.
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a lifecycle event hook, replacing the previous callback.
    ///
    /// Listeners run inline with execution and should remain short.
    ///
    /// # Type Parameters
    /// - `C`: Thread-safe consumer owned by this registration.
    ///
    /// # Parameters
    /// - `hook`: Hook receiving each emitted lifecycle event.
    ///
    /// # Returns
    /// The updated hook set.
    #[must_use]
    #[inline(always)]
    pub fn on_event<C>(mut self, hook: C) -> Self
    where
        C: Consumer<CasEvent> + Send + Sync + 'static,
    {
        self.on_event = Some(ArcConsumer::new(hook));
        self
    }

    /// Registers a contention alert hook and its thresholds.
    ///
    /// Replaces both the previous callback and its thresholds. All configured
    /// thresholds must be met before an alert is dispatched.
    ///
    /// # Type Parameters
    /// - `C`: Thread-safe consumer retained for this execution.
    ///
    /// # Parameters
    /// - `thresholds`: Minimum attempts, conflicts, and conflict ratio.
    /// - `hook`: Callback invoked inline after the execution finishes.
    ///
    /// # Returns
    /// The updated hook set. Listener panics are isolated in unwind builds.
    #[must_use]
    #[inline(always)]
    pub fn on_contention_alert<C>(mut self, thresholds: ContentionThresholds, hook: C) -> Self
    where
        C: Consumer<CasAlert> + Send + Sync + 'static,
    {
        self.on_alert = Some(ArcConsumer::new(hook));
        self.contention_thresholds = Some(thresholds);
        self
    }

    /// Returns the registered lifecycle event hook.
    ///
    /// # Returns
    /// `Some` cloned shared listener, or `None` when events are disabled.
    #[must_use]
    #[inline(always)]
    pub(crate) fn event_hook(&self) -> Option<CasEventHook> {
        self.on_event.clone()
    }

    /// Returns the registered alert hook.
    ///
    /// # Returns
    /// `Some` cloned shared listener, or `None` when alerts are disabled.
    #[must_use]
    #[inline(always)]
    pub(crate) fn alert_hook(&self) -> Option<CasAlertHook> {
        self.on_alert.clone()
    }

    /// Returns `Some` registered thresholds, or `None` when alerts are
    /// disabled.
    ///
    /// # Returns
    /// `Some` thresholds registered with the alert callback, or `None` for
    /// disabled alerts.
    #[must_use]
    #[inline(always)]
    pub(crate) fn contention_thresholds(&self) -> Option<ContentionThresholds> {
        self.contention_thresholds
    }
}
