// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lifecycle dispatch after retry scheduling has been accepted.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Mutex;

use qubit_retry::BackoffStep;
use qubit_retry::RetryContext;
use qubit_retry::RetryObserver;

use crate::error::CasAttemptFailure;
use crate::event::CasContext;
use crate::event::CasEvent;
use crate::event::CasEventHook;
use crate::executor::CasExecutor;
use crate::report::CasReportBuilder;

/// Bridges accepted retry schedules to CAS events without changing decisions.
///
/// # Type Parameters
/// - `T`: Shared application state.
/// - `E`: Business failure returned by an operation.
pub(in crate::executor::cas_executor) struct CasRetryObserver<T, E> {
    /// Shared lifecycle callback for this execution, if registered.
    event_hook: Option<CasEventHook>,
    /// Accumulator updated only after listener dispatch releases control.
    report_builder: Arc<Mutex<CasReportBuilder>>,
    /// Connects failure types without owning application values.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> CasRetryObserver<T, E> {
    /// Creates an observer sharing this execution's listener and report.
    ///
    /// # Parameters
    /// - `event_hook`: `Some` enables event dispatch; `None` disables it.
    /// - `report_builder`: Accumulator retaining isolated listener failures.
    ///
    /// # Returns
    /// An observer that never holds the report lock while invoking a listener.
    #[inline]
    #[must_use]
    pub(in crate::executor::cas_executor) fn new(
        event_hook: Option<CasEventHook>,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> Self {
        Self {
            event_hook,
            report_builder,
            marker: PhantomData,
        }
    }
}

impl<T, E> RetryObserver<CasAttemptFailure<T, E>> for CasRetryObserver<T, E>
where
    T: 'static,
    E: 'static,
{
    /// Dispatches a selected retry delay and retains any listener panic.
    ///
    /// # Parameters
    /// - `backoff`: Delay selected after scheduling checks pass.
    /// - `context`: Snapshot of the retry flow at scheduling time.
    ///
    /// # Panics
    /// Panics if the report mutex is poisoned; listener panics are isolated.
    fn on_retry_scheduled(&self, backoff: &BackoffStep, context: &RetryContext) {
        if CasExecutor::<T, E>::should_emit_events(&self.event_hook)
            && let Some(listener_failure) = CasExecutor::<T, E>::dispatch_event(
                self.event_hook
                    .as_ref()
                    .expect("event hook should exist when events are emitted"),
                CasEvent::RetryScheduled {
                    context: CasContext::new(context),
                    delay: backoff.effective_delay(),
                },
            )
        {
            self.report_builder
                .lock()
                .expect("CAS report builder should be lockable")
                .record_listener_failure(listener_failure);
        }
    }
}
