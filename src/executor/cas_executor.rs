// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS executor implementation.

#[path = "cas_executor/async_execution.rs"]
#[cfg(feature = "tokio")]
mod async_execution;
#[path = "cas_executor/decision.rs"]
mod decision;
#[path = "cas_executor/dispatch.rs"]
mod dispatch;
#[path = "cas_executor/finalization.rs"]
mod finalization;
#[path = "cas_executor/retry_adapter.rs"]
mod retry_adapter;
#[path = "cas_executor/sync_execution.rs"]
mod sync_execution;

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::OnceLock;

use qubit_error::BoxError;
use qubit_retry::Retry;
use qubit_retry::RetryPolicy;

use super::cas_builder::CasBuilder;
use super::internal::AttemptTimeoutAction;
use crate::error::CasAttemptFailure;
use crate::observability::CasObservabilityConfig;
use crate::strategy::CasStrategy;

/// Executor for retry-aware compare-and-swap workflows.
#[derive(Clone)]
pub struct CasExecutor<T, E = BoxError> {
    /// Pure policy used by the retry facades.
    policy: RetryPolicy,
    /// Optional hard wall-clock timeout for asynchronous retry flows.
    flow_timeout: Option<std::time::Duration>,
    /// Optional hard timeout applied to each async attempt.
    attempt_timeout: Option<std::time::Duration>,
    /// Action selected after a configured attempt timeout.
    attempt_timeout_action: AttemptTimeoutAction,
    /// Observability settings shared by executions.
    observability: CasObservabilityConfig,
    /// Result-only retry definition initialized on its first use.
    result_retry: Arc<OnceLock<Retry<CasAttemptFailure<T, E>>>>,
    /// Marker preserving `T` and `E`.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> std::fmt::Debug for CasExecutor<T, E> {
    /// Formats the reusable executor configuration without forcing retry rules
    /// or application types to implement [`std::fmt::Debug`].
    ///
    /// # Parameters
    /// - `f`: Formatter provided by the standard formatting machinery.
    ///
    /// # Returns
    /// `fmt::Result` from the formatter.
    ///
    /// # Errors
    /// Returns a formatting error if the formatter fails.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CasExecutor")
            .field("policy", &self.policy)
            .field("flow_timeout", &self.flow_timeout)
            .field("attempt_timeout", &self.attempt_timeout)
            .field("attempt_timeout_action", &self.attempt_timeout_action)
            .field("observability", &self.observability)
            .finish()
    }
}

impl<T, E> CasExecutor<T, E> {
    /// Creates a CAS builder.
    ///
    /// # Returns
    /// A builder configured with default retry settings.
    #[inline(always)]
    pub fn builder() -> CasBuilder<T, E> {
        CasBuilder::new()
    }

    /// Creates an executor from a pure retry policy.
    ///
    /// # Parameters
    /// - `policy`: Retry continuation and backoff policy to install.
    ///
    /// # Returns
    /// A configured executor using the supplied retry policy.
    pub fn from_policy(policy: RetryPolicy) -> Self {
        Self::builder()
            .policy(policy)
            .build()
            .expect("an existing retry policy is already validated")
    }

    /// Creates an executor tuned for low-latency workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    pub fn latency_first() -> Self {
        Self::builder()
            .build_latency_first()
            .expect("latency-first CAS strategy must be valid")
    }

    /// Creates an executor tuned for hot-contention workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    pub fn contention_adaptive() -> Self {
        Self::builder()
            .build_contention_adaptive()
            .expect("contention-adaptive CAS strategy must be valid")
    }

    /// Creates an executor tuned for reliability-first workloads.
    ///
    /// # Returns
    /// A configured executor. The built-in strategy is always valid.
    pub fn reliability_first() -> Self {
        Self::builder()
            .build_reliability_first()
            .expect("reliability-first CAS strategy must be valid")
    }

    /// Creates an executor from a built-in strategy.
    ///
    /// # Parameters
    /// - `strategy`: Strategy to install.
    ///
    /// # Returns
    /// A configured executor. Built-in strategies are always valid.
    pub fn with_strategy(strategy: CasStrategy) -> Self {
        Self::builder()
            .strategy(strategy)
            .build()
            .expect("built-in CAS strategy must be valid")
    }

    /// Creates one executor from validated parts.
    ///
    /// # Parameters
    /// - `policy`: Validated retry policy.
    /// - `attempt_timeout`: Optional hard timeout for async attempts.
    /// - `flow_timeout`: Optional hard timeout for asynchronous retry flows.
    /// - `attempt_timeout_action`: Action selected for attempt timeouts.
    /// - `observability`: Observability settings shared by executions.
    ///
    /// # Returns
    /// A configured executor.
    #[inline]
    pub(crate) fn new(
        policy: RetryPolicy,
        attempt_timeout: Option<std::time::Duration>,
        flow_timeout: Option<std::time::Duration>,
        attempt_timeout_action: AttemptTimeoutAction,
        observability: CasObservabilityConfig,
    ) -> Self {
        Self {
            policy,
            flow_timeout,
            attempt_timeout,
            attempt_timeout_action,
            observability,
            result_retry: Arc::new(OnceLock::new()),
            marker: PhantomData,
        }
    }

    /// Returns the immutable retry policy used by this executor.
    ///
    /// # Returns
    /// Shared retry policy.
    #[inline(always)]
    pub fn policy(&self) -> &RetryPolicy {
        &self.policy
    }

    /// Returns the optional hard timeout for each async attempt.
    #[inline(always)]
    pub fn attempt_timeout(&self) -> Option<std::time::Duration> {
        self.attempt_timeout
    }

    /// Returns the hard wall-clock boundary for asynchronous retry flows.
    ///
    /// # Returns
    /// The end-to-end total elapsed budget, when configured. The operation
    /// budget controls whether another attempt may start and never cancels an
    /// admitted attempt.
    #[inline(always)]
    pub fn flow_timeout(&self) -> Option<std::time::Duration> {
        self.flow_timeout
    }

    /// Returns observability settings used by this executor.
    ///
    /// # Returns
    /// Shared observability configuration.
    #[inline(always)]
    pub fn observability(&self) -> &CasObservabilityConfig {
        &self.observability
    }
}
