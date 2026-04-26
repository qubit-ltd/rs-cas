/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Builder for [`crate::CasExecutor`].

use std::marker::PhantomData;
use std::time::Duration;

use qubit_common::BoxError;
use qubit_retry::{RetryConfigError, RetryDelay, RetryJitter, RetryOptions};

use crate::observability::{CasObservabilityConfig, ContentionThresholds, ListenerPanicPolicy};
use crate::options::CasTimeoutPolicy;
use crate::strategy::CasStrategy;

use super::cas_executor::CasExecutor;

/// Builder for [`CasExecutor`](crate::CasExecutor).
pub struct CasBuilder<T, E = BoxError> {
    /// Maximum attempts, including the initial attempt.
    max_attempts: u32,
    /// Maximum total elapsed time.
    max_elapsed: Option<Duration>,
    /// Base retry delay strategy.
    delay: RetryDelay,
    /// Jitter strategy applied to the base delay.
    jitter: RetryJitter,
    /// Optional async attempt timeout.
    attempt_timeout: Option<Duration>,
    /// Policy used when one attempt exceeds the timeout.
    timeout_policy: CasTimeoutPolicy,
    /// Observability settings.
    observability: CasObservabilityConfig,
    /// Stored validation error for zero max attempts.
    max_attempts_error: Option<RetryConfigError>,
    /// Marker preserving the executor type parameters.
    marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> CasBuilder<T, E> {
    /// Creates a builder with default retry options.
    ///
    /// # Returns
    /// A [`CasBuilder`] using [`RetryOptions::default`].
    pub fn new() -> Self {
        let options = RetryOptions::default();
        Self {
            max_attempts: options.max_attempts(),
            max_elapsed: options.max_elapsed(),
            delay: options.delay().clone(),
            jitter: options.jitter(),
            attempt_timeout: None,
            timeout_policy: CasTimeoutPolicy::Retry,
            observability: CasObservabilityConfig::default(),
            max_attempts_error: None,
            marker: PhantomData,
        }
    }

    /// Replaces builder settings from an existing retry option snapshot.
    ///
    /// # Parameters
    /// - `options`: Retry option snapshot to install.
    ///
    /// # Returns
    /// The updated builder.
    pub fn options(mut self, options: RetryOptions) -> Self {
        self.max_attempts = options.max_attempts();
        self.max_elapsed = options.max_elapsed();
        self.delay = options.delay().clone();
        self.jitter = options.jitter();
        self.max_attempts_error = None;
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
        if max_attempts == 0 {
            self.max_attempts_error = Some(RetryConfigError::invalid_value(
                qubit_retry::constants::KEY_MAX_ATTEMPTS,
                "max_attempts must be greater than zero",
            ));
        } else {
            self.max_attempts = max_attempts;
            self.max_attempts_error = None;
        }
        self
    }

    /// Sets the maximum retries after the initial attempt.
    ///
    /// # Parameters
    /// - `max_retries`: Maximum retries after the first attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn max_retries(self, max_retries: u32) -> Self {
        self.max_attempts(max_retries.saturating_add(1))
    }

    /// Sets the maximum elapsed-time budget.
    ///
    /// # Parameters
    /// - `max_elapsed`: Optional total elapsed-time budget.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn max_elapsed(mut self, max_elapsed: Option<Duration>) -> Self {
        self.max_elapsed = max_elapsed;
        self
    }

    /// Sets the base retry delay strategy.
    ///
    /// # Parameters
    /// - `delay`: Delay strategy used between failed attempts.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn delay(mut self, delay: RetryDelay) -> Self {
        self.delay = delay;
        self
    }

    /// Uses immediate retries with no sleep.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn no_delay(self) -> Self {
        self.delay(RetryDelay::none())
    }

    /// Uses one fixed retry delay.
    ///
    /// # Parameters
    /// - `delay`: Delay slept before each retry.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn fixed_delay(self, delay: Duration) -> Self {
        self.delay(RetryDelay::fixed(delay))
    }

    /// Uses one random retry delay range.
    ///
    /// # Parameters
    /// - `min`: Inclusive minimum delay.
    /// - `max`: Inclusive maximum delay.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn random_delay(self, min: Duration, max: Duration) -> Self {
        self.delay(RetryDelay::random(min, max))
    }

    /// Uses exponential backoff with multiplier `2.0`.
    ///
    /// # Parameters
    /// - `initial`: Initial retry delay.
    /// - `max`: Maximum retry delay.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
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
    #[inline]
    pub fn exponential_backoff_with_multiplier(
        self,
        initial: Duration,
        max: Duration,
        multiplier: f64,
    ) -> Self {
        self.delay(RetryDelay::exponential(initial, max, multiplier))
    }

    /// Sets the jitter strategy.
    ///
    /// # Parameters
    /// - `jitter`: Jitter strategy applied to retry delays.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn jitter(mut self, jitter: RetryJitter) -> Self {
        self.jitter = jitter;
        self
    }

    /// Sets relative jitter by factor.
    ///
    /// # Parameters
    /// - `factor`: Relative jitter factor in `[0.0, 1.0]`.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn jitter_factor(self, factor: f64) -> Self {
        self.jitter(RetryJitter::factor(factor))
    }

    /// Sets the async per-attempt timeout.
    ///
    /// # Parameters
    /// - `attempt_timeout`: Timeout applied to each async CAS attempt.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn attempt_timeout(mut self, attempt_timeout: Option<Duration>) -> Self {
        self.attempt_timeout = attempt_timeout;
        self
    }

    /// Retries attempts that exceed the configured timeout.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn retry_on_timeout(mut self) -> Self {
        self.timeout_policy = CasTimeoutPolicy::Retry;
        self
    }

    /// Aborts the CAS flow when one attempt exceeds the timeout.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
    pub fn abort_on_timeout(mut self) -> Self {
        self.timeout_policy = CasTimeoutPolicy::Abort;
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
            .max_elapsed(Some(profile.max_elapsed()));
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
    #[inline]
    pub fn observability(mut self, observability: CasObservabilityConfig) -> Self {
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
    #[inline]
    pub fn alert_on_contention(mut self, thresholds: ContentionThresholds) -> Self {
        self.observability = self.observability.with_contention_thresholds(thresholds);
        self
    }

    /// Enables retry-layer listener panic isolation.
    ///
    /// # Returns
    /// The updated builder.
    #[inline]
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
    /// Returns [`RetryConfigError`] when the configured retry settings are
    /// invalid.
    pub fn build(self) -> Result<CasExecutor<T, E>, RetryConfigError> {
        if let Some(error) = self.max_attempts_error {
            return Err(error);
        }
        let options =
            RetryOptions::new(self.max_attempts, self.max_elapsed, self.delay, self.jitter)?;
        Ok(CasExecutor::new(
            options,
            self.attempt_timeout,
            self.timeout_policy,
            self.observability,
        ))
    }

    /// Builds one executor with the contention-adaptive strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] suitable for contended writers.
    pub fn build_contention_adaptive(self) -> Result<CasExecutor<T, E>, RetryConfigError> {
        self.strategy(CasStrategy::ContentionAdaptive).build()
    }

    /// Builds one executor with the latency-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for low latency.
    pub fn build_latency_first(self) -> Result<CasExecutor<T, E>, RetryConfigError> {
        self.strategy(CasStrategy::LatencyFirst).build()
    }

    /// Builds one executor with the reliability-first strategy.
    ///
    /// # Returns
    /// A configured [`CasExecutor`] optimized for long retry windows.
    pub fn build_reliability_first(self) -> Result<CasExecutor<T, E>, RetryConfigError> {
        self.strategy(CasStrategy::ReliabilityFirst).build()
    }
}

impl<T, E> Default for CasBuilder<T, E> {
    /// Creates a default CAS builder.
    ///
    /// # Returns
    /// A builder equivalent to [`CasBuilder::new`].
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
