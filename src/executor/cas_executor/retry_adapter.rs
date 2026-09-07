// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Retry policy construction for CAS executions.

use std::sync::Arc;
use std::sync::Mutex;

use qubit_retry::AttemptFailure;
use qubit_retry::Retry;
use qubit_retry::RetryContext;
use qubit_retry::RetryDecision;
use qubit_retry::RetryFallback;

use super::CasExecutor;
use crate::error::CasAttemptFailure;
use crate::event::CasContext;
use crate::event::CasEvent;
use crate::event::CasHooks;
use crate::executor::cas_executor::retry_adapter;
use crate::executor::internal::AttemptTimeoutAction;
use crate::report::CasReportBuilder;

/// Classifies one retry-layer failure without producing side effects.
pub(super) fn retry_decision<T, E>(
    failure: &AttemptFailure<CasAttemptFailure<T, E>>,
    attempt_timeout_action: AttemptTimeoutAction,
) -> RetryDecision {
    use qubit_retry::RetryTimeoutScope;
    match failure {
        AttemptFailure::Error(CasAttemptFailure::Conflict { .. })
        | AttemptFailure::Error(CasAttemptFailure::Retry { .. }) => RetryDecision::Retry,
        AttemptFailure::Error(CasAttemptFailure::Abort { .. }) => RetryDecision::Abort,
        AttemptFailure::Error(CasAttemptFailure::Timeout { .. }) => RetryDecision::UseDefault,
        AttemptFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt,
        } if attempt_timeout_action == AttemptTimeoutAction::Retry => RetryDecision::Retry,
        AttemptFailure::TimedOut { .. } | AttemptFailure::Panicked { .. } => RetryDecision::UseDefault,
        _ => RetryDecision::UseDefault,
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Builds one retry policy for a single CAS execution.
    ///
    /// # Parameters
    /// - `hooks`: Hook registrations for the current execution.
    /// # Returns
    /// A retry policy configured for one CAS execution.
    pub(super) fn build_retry(
        &self,
        hooks: &CasHooks,
        report_builder: Arc<Mutex<CasReportBuilder>>,
    ) -> Retry<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        let event_hook = hooks.event_hook();
        let attempt_timeout_action = self.attempt_timeout_action;
        let observability = self.observability.clone();
        let observer_event_hook = event_hook.clone();
        let observer_observability = observability.clone();
        let observer_report_builder = Arc::clone(&report_builder);

        Retry::<CasAttemptFailure<T, E>>::builder(self.policy.clone())
            .fallback(RetryFallback::Retry)
            .observer(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, context: &RetryContext| {
                    let kind = match failure {
                        AttemptFailure::Error(failure) => {
                            let mut report = observer_report_builder
                                .lock()
                                .expect("CAS report builder should be lockable");
                            match failure {
                                CasAttemptFailure::Conflict { .. } => report.record_conflict(),
                                CasAttemptFailure::Retry { .. } => report.record_retry_error(),
                                CasAttemptFailure::Abort { .. } => report.record_abort(),
                                CasAttemptFailure::Timeout { .. } => report.record_timeout(),
                            }
                            Some(Self::failure_kind(failure))
                        }
                        AttemptFailure::TimedOut { .. } => {
                            observer_report_builder
                                .lock()
                                .expect("CAS report builder should be lockable")
                                .record_timeout();
                            Some(crate::error::CasAttemptFailureKind::Timeout)
                        }
                        AttemptFailure::Panicked { .. } => None,
                        _ => None,
                    };
                    if let Some(kind) = kind
                        && Self::should_emit_events(&observer_observability, &observer_event_hook)
                    {
                        Self::dispatch_event(
                            &observer_observability,
                            observer_event_hook
                                .as_ref()
                                .expect("event hook should exist when events are emitted"),
                            CasEvent::AttemptFailed {
                                context: CasContext::new(context),
                                kind,
                            },
                        );
                    }
                },
            )
            .rule(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, context: &RetryContext| {
                    let decision = super::retry_adapter::retry_decision(failure, attempt_timeout_action);
                    if matches!(decision, RetryDecision::Retry) && Self::should_emit_events(&observability, &event_hook)
                    {
                        Self::dispatch_event(
                            &observability,
                            event_hook
                                .as_ref()
                                .expect("event hook should exist when events are emitted"),
                            CasEvent::RetryRequested {
                                context: CasContext::new(context),
                            },
                        );
                    }
                    decision
                },
            )
            .build()
    }

    /// Returns the cached retry definition used by result-only execution.
    ///
    /// # Returns
    /// An immutable retry definition initialized exactly once per executor.
    pub(super) fn result_retry(&self) -> &Retry<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        self.result_retry.get_or_init(|| {
            let attempt_timeout_action = self.attempt_timeout_action;
            Retry::<CasAttemptFailure<T, E>>::builder(self.policy.clone())
                .fallback(RetryFallback::Retry)
                .rule(
                    move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, _context: &RetryContext| {
                        retry_adapter::retry_decision(failure, attempt_timeout_action)
                    },
                )
                .build()
        })
    }
}
