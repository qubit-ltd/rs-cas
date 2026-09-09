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
use qubit_retry::BackoffStep;
use qubit_retry::RetryConfig;
use qubit_retry::RetryContext;
use qubit_retry::RetryDecision;
use qubit_retry::RetryFallback;
use qubit_retry::RetryObserver;

use super::CasExecutor;
use crate::error::CasAttemptFailure;
use crate::event::CasContext;
use crate::event::CasEvent;
use crate::event::CasHooks;
use crate::executor::cas_executor::retry_adapter;
use crate::executor::internal::AttemptTimeoutAction;
use crate::report::CasReportBuilder;

struct CasRetryObserver<T, E> {
    event_hook: Option<crate::event::CasEventHook>,
    report_builder: Arc<Mutex<CasReportBuilder>>,
    _marker: std::marker::PhantomData<fn() -> (T, E)>,
}

impl<T, E> RetryObserver<CasAttemptFailure<T, E>> for CasRetryObserver<T, E>
where
    T: 'static,
    E: 'static,
{
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
    ) -> RetryConfig<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        let event_hook = hooks.event_hook();
        let attempt_timeout_action = self.attempt_timeout_action;
        let observer_event_hook = event_hook.clone();
        let observer_report_builder = Arc::clone(&report_builder);

        RetryConfig::<CasAttemptFailure<T, E>>::builder()
            .policy(self.policy.clone())
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
                        && Self::should_emit_events(&observer_event_hook)
                        && let Some(listener_failure) = Self::dispatch_event(
                            observer_event_hook
                                .as_ref()
                                .expect("event hook should exist when events are emitted"),
                            CasEvent::AttemptFailed {
                                context: CasContext::new(context),
                                kind,
                            },
                        )
                    {
                        observer_report_builder
                            .lock()
                            .expect("CAS report builder should be lockable")
                            .record_listener_failure(listener_failure);
                    }
                },
            )
            .observer(CasRetryObserver {
                event_hook,
                report_builder,
                _marker: std::marker::PhantomData,
            })
            .rule(
                move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, context: &RetryContext| {
                    let _ = context;
                    super::retry_adapter::retry_decision(failure, attempt_timeout_action)
                },
            )
            .build()
            .expect("validated CAS retry configuration")
    }

    /// Returns the cached retry definition used by result-only execution.
    ///
    /// # Returns
    /// An immutable retry definition initialized exactly once per executor.
    pub(super) fn result_retry(&self) -> &RetryConfig<CasAttemptFailure<T, E>>
    where
        T: 'static,
        E: 'static,
    {
        self.result_retry.get_or_init(|| {
            let attempt_timeout_action = self.attempt_timeout_action;
            RetryConfig::<CasAttemptFailure<T, E>>::builder()
                .policy(self.policy.clone())
                .fallback(RetryFallback::Retry)
                .rule(
                    move |failure: &AttemptFailure<CasAttemptFailure<T, E>>, _context: &RetryContext| {
                        retry_adapter::retry_decision(failure, attempt_timeout_action)
                    },
                )
                .build()
                .expect("validated CAS retry configuration")
        })
    }
}
