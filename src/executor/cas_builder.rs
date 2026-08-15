// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for [`crate::CasExecutor`].

use std::marker::PhantomData;
use std::time::Duration;

use qubit_error::BoxError;
use qubit_retry::BackoffPolicy;
use qubit_retry::RetryPolicy;
use qubit_retry::RetryPolicyError;

use super::cas_executor::CasExecutor;
use super::internal::AttemptTimeoutAction;
use crate::constants::DEFAULT_CAS_MAX_ATTEMPTS;
use crate::observability::CasObservabilityConfig;
use crate::observability::ContentionThresholds;
use crate::observability::ListenerPanicPolicy;
use crate::strategy::CasStrategy;

/// Builder for [`CasExecutor`](crate::CasExecutor).
///
/// ```compile_fail
/// #![deny(unused_must_use)]
///
/// use qubit_cas::CasExecutor;
///
/// CasExecutor::<usize, ()>::builder();
/// ```
#[must_use = "a CAS builder must be configured or built"]
pub struct CasBuilder<T, E = BoxError> {
    /// Maximum total attempts, including the initial operation.
    max_attempts: u32,
    /// Optional cumulative operation-time continuation budget.
    max_operation_elapsed: Option<Duration>,
    /// Optional end-to-end continuation budget.
    max_total_elapsed: Option<Duration>,
    /// Validated backoff policy or its deferred construction error.
    backoff: Result<BackoffPolicy, RetryPolicyError>,
    /// Optional hard timeout applied to each async attempt.
    attempt_timeout: Option<Duration>,
    /// Action selected after a configured attempt timeout.
    attempt_timeout_action: AttemptTimeoutAction,
    /// Observability settings.
    observability: CasObservabilityConfig,
    /// Marker preserving the executor type parameters.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> CasBuilder<T, E> {
    /// Creates a builder with default retry policy values.
    ///
    /// # Returns
    /// A [`CasBuilder`] using immediate retries and the CAS default limit.
    pub fn new() -> Self {
        Self {
            max_attempts: DEFAULT_CAS_MAX_ATTEMPTS,
            max_operation_elapsed: None,
            max_total_elapsed: None,
            backoff: Ok(BackoffPolicy::immediate()),
            attempt_timeout: None,
            attempt_timeout_action: AttemptTimeoutAction::Abort,
            observability: CasObservabilityConfig::default(),
            marker: PhantomData,
        }
    }

    /// Replaces the pure retry policy used by the executor.
    ///
    /// # Parameters
    /// - `policy`: Retry continuation and backoff policy to install.
    ///
    /// # Returns
    /// The updated builder.
    pub fn policy(mut self, policy: RetryPolicy) -> Self {
        self.max_attempts = policy.limits().max_attempts().get();
        self.max_operation_elapsed = policy.limits().max_operation_elapsed();
        self.max_total_elapsed = policy.limits().max_total_elapsed();
        self.backoff = Ok(policy.backoff().clone());
        self
    }

    /// Sets the maximum total attempts.
    ///
    /// # Parameters
    /// - `max_attempts`: Maximum attempts, including the initial attempt.
    ///
    /// # Returns
    /// The updated builder.
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets the maximum retries after the initial attempt.
    ///
    /// # Parameters
    /// - `max_retries`: Maximum retries after the first attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_retries(self, max_retries: u32) -> Self {
        self.max_attempts(max_retries.saturating_add(1))
    }

    /// Sets the maximum cumulative user operation elapsed-time budget.
    ///
    /// # Parameters
    /// - `max_operation_elapsed`: Optional cumulative user operation time
    ///   budget.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_operation_elapsed(
        mut self,
        max_operation_elapsed: Option<Duration>,
    ) -> Self {
        self.max_operation_elapsed = max_operation_elapsed;
        self
    }

    /// Sets the maximum monotonic elapsed-time budget for the whole retry flow.
    ///
    /// # Parameters
    /// - `max_total_elapsed`: Optional total retry-flow time budget.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn max_total_elapsed(
        mut self,
        max_total_elapsed: Option<Duration>,
    ) -> Self {
        self.max_total_elapsed = max_total_elapsed;
        self
    }

    /// Sets the complete retry backoff policy.
    ///
    /// # Parameters
    /// - `backoff`: Backoff and jitter policy used between attempts.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn backoff(mut self, backoff: BackoffPolicy) -> Self {
        self.backoff = Ok(backoff);
        self
    }

    /// Uses immediate retries with no sleep.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn no_delay(self) -> Self {
        self.backoff(BackoffPolicy::immediate())
    }

    /// Uses one fixed retry delay.
    ///
    /// # Parameters
    /// - `delay`: Delay slept before each retry.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn fixed_delay(self, delay: Duration) -> Self {
        self.backoff(BackoffPolicy::fixed(delay))
    }

    /// Uses one random retry delay range.
    ///
    /// # Parameters
    /// - `min`: Inclusive minimum delay.
    /// - `max`: Inclusive maximum delay.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn random_delay(mut self, min: Duration, max: Duration) -> Self {
        self.backoff = BackoffPolicy::uniform(min, max);
        self
    }

    /// Uses exponential backoff with multiplier `2.0`.
    ///
    /// # Parameters
    /// - `initial`: Initial retry delay.
    /// - `max`: Maximum retry delay.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn exponential_backoff(self, initial: Duration, max: Duration) -> Self {
        self.exponential_backoff_with_multiplier(initial, max, 2.0)
    }

    /// Uses exponential backoff with a custom multiplier.
    ///
    /// # Parameters
    /// - `initial`: Initial retry delay.
    /// - `max`: Maximum retry delay.
    /// - `multiplier`: Multiplier applied after each failed attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn exponential_backoff_with_multiplier(
        self,
        initial: Duration,
        max: Duration,
        multiplier: f64,
    ) -> Self {
        let mut builder = self;
        builder.backoff = BackoffPolicy::exponential(initial, multiplier, max);
        builder
    }

    /// Sets relative jitter by factor.
    ///
    /// # Parameters
    /// - `factor`: Relative jitter factor in `[0.0, 1.0]`.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn jitter_factor(mut self, factor: f64) -> Self {
        self.backoff = self
            .backoff
            .and_then(|backoff| backoff.with_bounded_jitter(factor));
        self
    }

    /// Sets the async per-attempt timeout.
    ///
    /// # Parameters
    /// - `attempt_timeout`: Timeout applied to each async CAS attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn attempt_timeout(
        mut self,
        attempt_timeout: Option<Duration>,
    ) -> Self {
        self.attempt_timeout = attempt_timeout;
        self
    }

    /// Retries attempts that exceed the configured timeout.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn retry_on_timeout(mut self) -> Self {
        self.attempt_timeout_action = AttemptTimeoutAction::Retry;
        self
    }

    /// Aborts the CAS flow when one attempt exceeds the timeout.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn abort_on_timeout(mut self) -> Self {
        self.attempt_timeout_action = AttemptTimeoutAction::Abort;
        self
    }

    /// Applies a built-in CAS strategy to this builder.
    ///
    /// # Parameters
    /// - `strategy`: Strategy profile to install.
    ///
    /// # Returns
    /// The updated builder.
    pub fn strategy(self, strategy: CasStrategy) -> Self {
        let profile = strategy.profile();
        let builder = self
            .max_attempts(profile.max_attempts())
            .max_operation_elapsed(Some(profile.max_operation_elapsed()))
            .max_total_elapsed(profile.max_total_elapsed());
        if let Some((initial, max, jitter)) = strategy.backoff() {
            builder
                .exponential_backoff(initial, max)
                .jitter_factor(jitter)
        } else {
            builder.no_delay()
        }
    }

    /// Installs observability configuration.
    ///
    /// # Parameters
    /// - `observability`: Observability settings to use.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn observability(
        mut self,
        observability: CasObservabilityConfig,
    ) -> Self {
        self.observability = observability;
        self
    }

    /// Enables contention alerting with the supplied thresholds.
    ///
    /// # Parameters
    /// - `thresholds`: Thresholds used to classify hot contention.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn alert_on_contention(
        mut self,
        thresholds: ContentionThresholds,
    ) -> Self {
        self.observability =
            self.observability.with_contention_thresholds(thresholds);
        self
    }

    /// Catches listener panics at dispatch instead of exposing them to their
    /// owning execution boundaries.
    ///
    /// # Returns
    /// The updated builder.
    #[inline(always)]
    pub fn isolate_listener_panics(mut self) -> Self {
        self.observability = self
            .observability
            .with_listener_panic_policy(ListenerPanicPolicy::Isolate);
        self
    }

    /// Builds one executor after validating the settings.
    ///
    /// # Returns
    /// A validated [`CasExecutor`].
    ///
    /// # Errors
    /// Returns [`RetryPolicyError`] when the configured retry settings are
    /// invalid.
    pub fn build(self) -> Result<CasExecutor<T, E>, RetryPolicyError> {
        let backoff = self.backoff?;
        let policy = RetryPolicy::builder()
            .max_attempts(self.max_attempts)
            .max_operation_elapsed_opt(self.max_operation_elapsed)
            .max_total_elapsed_opt(self.max_total_elapsed)
            .backoff(backoff)
            .build()?;
        Ok(CasExecutor::new(
            policy,
            self.attempt_timeout,
            self.attempt_timeout_action,
            self.observability,
        ))
    }

    /// Builds one executor with the contention-adaptive strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] suitable for contended writers.
    pub fn build_contention_adaptive(
        self,
    ) -> Result<CasExecutor<T, E>, RetryPolicyError> {
        self.strategy(CasStrategy::ContentionAdaptive).build()
    }

    /// Builds one executor with the latency-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for low latency.
    pub fn build_latency_first(
        self,
    ) -> Result<CasExecutor<T, E>, RetryPolicyError> {
        self.strategy(CasStrategy::LatencyFirst).build()
    }

    /// Builds one executor with the reliability-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for long retry windows.
    pub fn build_reliability_first(
        self,
    ) -> Result<CasExecutor<T, E>, RetryPolicyError> {
        self.strategy(CasStrategy::ReliabilityFirst).build()
    }
}

impl<T, E> Default for CasBuilder<T, E> {
    /// Creates a default CAS builder.
    ///
    /// # Returns
    /// A builder equivalent to [`CasBuilder::new`].
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
