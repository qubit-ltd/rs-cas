// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS event and hook registrations.

use qubit_function::{ArcConsumer, Consumer};

use crate::observability::CasAlert;

use super::{CasAlertHook, CasEvent, CasEventHook};

/// Per-execution hooks for observing CAS lifecycle events.
#[derive(Clone)]
pub struct CasHooks {
    /// Hook invoked for lifecycle events.
    on_event: Option<CasEventHook>,
    /// Hook invoked when configured alert thresholds are crossed.
    on_alert: Option<CasAlertHook>,
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
        }
    }
}

impl CasHooks {
    /// Creates an empty hook set.
    ///
    /// # Returns
    /// A [`CasHooks`] value with every hook unset.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a lifecycle event hook.
    ///
    /// # Parameters
    /// - `hook`: Hook receiving each emitted lifecycle event.
    ///
    /// # Returns
    /// The updated hook set.
    #[inline(always)]
    pub fn on_event<C>(mut self, hook: C) -> Self
    where
        C: Consumer<CasEvent> + Send + Sync + 'static,
    {
        self.on_event = Some(ArcConsumer::new(hook));
        self
    }

    /// Registers an alert hook.
    ///
    /// # Parameters
    /// - `hook`: Hook receiving contention alerts.
    ///
    /// # Returns
    /// The updated hook set.
    #[inline(always)]
    pub fn on_alert<C>(mut self, hook: C) -> Self
    where
        C: Consumer<CasAlert> + Send + Sync + 'static,
    {
        self.on_alert = Some(ArcConsumer::new(hook));
        self
    }

    /// Returns the registered lifecycle event hook.
    ///
    /// # Returns
    /// Optional shared event hook.
    #[inline(always)]
    pub(crate) fn event_hook(&self) -> Option<CasEventHook> {
        self.on_event.clone()
    }

    /// Returns the registered alert hook.
    ///
    /// # Returns
    /// Optional shared alert hook.
    #[inline(always)]
    pub(crate) fn alert_hook(&self) -> Option<CasAlertHook> {
        self.on_alert.clone()
    }
}
