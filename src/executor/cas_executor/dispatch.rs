// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lifecycle dispatch and event wiring for CAS executions.

use std::sync::Arc;
use std::sync::Mutex;

use super::CasExecutor;
use crate::error::CasAttemptFailure;
use crate::event::CasEvent;
use crate::event::CasHooks;
use crate::event::CasListenerFailure;
use crate::event::CasListenerKind;
use crate::executor::internal::CasReportFinishContext;
use crate::observability::CasAlert;
use crate::report::CasExecutionReport;
use crate::report::CasReportBuilder;

/// Returns whether lifecycle events should be emitted.
pub(super) fn should_emit_events(hook: &Option<crate::event::CasEventHook>) -> bool {
    hook.is_some()
}

/// Dispatches one lifecycle event according to listener panic policy.
pub(super) fn dispatch_event(hook: &crate::event::CasEventHook, event: CasEvent) -> Option<CasListenerFailure> {
    use std::panic::AssertUnwindSafe;
    use std::panic::catch_unwind;

    use qubit_function::Consumer;
    catch_unwind(AssertUnwindSafe(|| hook.accept(&event)))
        .err()
        .map(|payload| CasListenerFailure::from_panic(listener_kind(&event), payload))
}

/// Dispatches one optional alert according to listener panic policy.
pub(super) fn dispatch_alert(hook: &Option<crate::event::CasAlertHook>, alert: CasAlert) -> Option<CasListenerFailure> {
    use std::panic::AssertUnwindSafe;
    use std::panic::catch_unwind;

    use qubit_function::Consumer;
    if let Some(hook) = hook {
        return catch_unwind(AssertUnwindSafe(|| hook.accept(&alert)))
            .err()
            .map(|payload| CasListenerFailure::from_panic(CasListenerKind::ContentionAlert, payload));
    }
    None
}

fn listener_kind(event: &CasEvent) -> CasListenerKind {
    match event {
        CasEvent::ExecutionStarted { .. } => CasListenerKind::ExecutionStarted,
        CasEvent::AttemptFailed { .. } => CasListenerKind::AttemptFailed,
        CasEvent::RetryScheduled { .. } => CasListenerKind::RetryScheduled,
        CasEvent::ExecutionFinished { .. } => CasListenerKind::ExecutionFinished,
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Emits the execution-started event when event streaming is enabled.
    ///
    /// # Parameters
    /// - `hooks`: Per-execution hooks (checked for event hook presence).
    /// - `report_builder`: Used to obtain the start instant for the event.
    pub(super) fn emit_started(&self, hooks: &CasHooks, report_builder: &Arc<Mutex<CasReportBuilder>>)
    where
        T: 'static,
        E: 'static,
    {
        if hooks.event_hook().is_none() {
            return;
        }
        let started_at = report_builder
            .lock()
            .expect("CAS report builder should be lockable")
            .started_at();
        let event_hook = hooks.event_hook();
        if let Some(failure) = Self::dispatch_event(
            event_hook
                .as_ref()
                .expect("event hook should exist when events are emitted"),
            CasEvent::ExecutionStarted { started_at },
        ) {
            report_builder
                .lock()
                .expect("CAS report builder should be lockable")
                .record_listener_failure(failure);
        }
    }

    /// Finishes and emits one execution report (and optional alert).
    ///
    /// Locks the report builder, finalizes the report, emits the
    /// `ExecutionFinished` event if enabled, and dispatches a contention alert
    /// if the mode and thresholds warrant it.
    ///
    /// # Parameters
    /// - `hooks`: Used for event and alert dispatching.
    /// - `report_builder`: Accumulator to finalize.
    /// - `ctx`: Retry limits and terminal outcome for the report.
    ///
    /// # Returns
    /// The finalized [`CasExecutionReport`].
    pub(super) fn finish_report(
        &self,
        hooks: &CasHooks,
        report_builder: Arc<Mutex<CasReportBuilder>>,
        ctx: CasReportFinishContext,
    ) -> CasExecutionReport
    where
        T: 'static,
        E: 'static,
    {
        let report = report_builder
            .lock()
            .expect("CAS report builder should be lockable")
            .finish(
                ctx.attempts_total,
                ctx.max_attempts,
                ctx.max_operation_elapsed,
                ctx.max_total_elapsed,
                ctx.outcome,
            );
        let event_hook = hooks.event_hook();
        if Self::should_emit_events(&event_hook) {
            let listener_failure = Self::dispatch_event(
                event_hook
                    .as_ref()
                    .expect("event hook should exist when events are emitted"),
                CasEvent::ExecutionFinished { report: report.clone() },
            );
            if let Some(failure) = listener_failure {
                report_builder
                    .lock()
                    .expect("CAS report builder should be lockable")
                    .record_listener_failure(failure);
            }
        }
        let alert_hook = hooks.alert_hook();
        if let Some(thresholds) = hooks.contention_thresholds()
            && alert_hook.is_some()
            && report.is_contention_hot(&thresholds)
            && let Some(failure) = Self::dispatch_alert(&alert_hook, CasAlert::contention(report.clone(), thresholds))
        {
            report_builder
                .lock()
                .expect("CAS report builder should be lockable")
                .record_listener_failure(failure);
        }
        let failures = report_builder
            .lock()
            .expect("CAS report builder should be lockable")
            .listener_failures();
        report.with_listener_failures(failures)
    }

    /// Converts one attempt failure into its lightweight event kind.
    ///
    /// # Parameters
    /// - `failure`: The [`CasAttemptFailure`] to classify.
    ///
    /// # Returns
    /// The [`CasAttemptFailureKind`] for event emission.
    #[inline]
    pub(super) fn failure_kind(failure: &CasAttemptFailure<T, E>) -> crate::error::CasAttemptFailureKind {
        failure.kind()
    }

    /// Dispatches one lifecycle event to a registered hook.
    ///
    /// # Parameters
    /// - `hook`: Listener that receives the event.
    /// - `event`: Lifecycle event to dispatch.
    ///
    /// # Panics
    /// Listener panics are isolated and do not change the CAS result.
    pub(super) fn dispatch_event(hook: &crate::event::CasEventHook, event: CasEvent) -> Option<CasListenerFailure>
    where
        T: 'static,
        E: 'static,
    {
        super::dispatch::dispatch_event(hook, event)
    }

    /// Returns whether lifecycle event construction and dispatch are needed.
    #[inline]
    pub(super) fn should_emit_events(hook: &Option<crate::event::CasEventHook>) -> bool {
        super::dispatch::should_emit_events(hook)
    }

    /// Dispatches one alert if an alert listener is registered.
    ///
    /// # Parameters
    /// - `hook`: Optional listener that receives the alert.
    /// - `alert`: Contention alert to dispatch.
    ///
    /// # Panics
    /// Exposes alert listener panics to the owning CAS execution boundary when
    /// Listener panics are isolated and recorded in the report.
    pub(super) fn dispatch_alert(
        hook: &Option<crate::event::CasAlertHook>,
        alert: CasAlert,
    ) -> Option<CasListenerFailure> {
        super::dispatch::dispatch_alert(hook, alert)
    }
}
