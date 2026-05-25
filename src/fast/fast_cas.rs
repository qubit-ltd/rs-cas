/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Compare-and-swap executor for compact [`usize`] state codes.
//!
//! The main type is [`FastCas`]. See the [`crate::fast`] module documentation
//! for how this layer relates to [`crate::CasExecutor`] and which APIs honor
//! [`FastCasPolicy`] retries.

use std::convert::Infallible;
use std::thread;

use super::{
    FastCasDecision,
    FastCasError,
    FastCasPolicy,
    FastCasState,
    FastCasSuccess,
};

/// Ultra-light compare-and-swap executor for `usize` state codes.
///
/// Use `FastCas` on hot paths where shared state is a compact integer code (for
/// example a small state-machine phase or lock word). The executor:
///
/// - Stores only a [`FastCasPolicy`] (`Copy`, no heap).
/// - Retries when [`FastCasState::compare_set`] fails because another writer
///   changed the value between your read and CAS (**conflict**). Business
///   errors returned as [`FastCasDecision::Abort`] do **not** trigger retries.
/// - Expects the operation closure passed to [`Self::execute`] /
///   [`Self::update_by`] to be safe to call again after conflicts: re-read the
///   current code from the closure argument each time; avoid non-idempotent side
///   effects inside the closure.
///
/// [`FastCas::compare_update`] and [`FastCas::compare_update_with`] perform a
/// **single** CAS attempt per call and **ignore** the configured policy (no
/// spin or retry loop).
///
/// # Examples
///
/// Pick a constructor, share one [`FastCasState`] across threads, then run
/// [`Self::execute`], [`Self::update_by`], or a compare-update helper.
///
/// ```
/// use qubit_cas::{FastCas, FastCasDecision, FastCasState};
///
/// const IDLE: usize = 0;
/// const BUSY: usize = 1;
///
/// let cas = FastCas::spin(32);
/// let state = FastCasState::new(IDLE);
///
/// let ok = cas
///     .execute(&state, |current| {
///         if current == IDLE {
///             FastCasDecision::update(BUSY, "go")
///         } else {
///             FastCasDecision::abort("not idle")
///         }
///     })
///     .unwrap();
///
/// assert_eq!(ok.current(), BUSY);
/// assert_eq!(*ok.output(), "go");
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FastCas {
    /// Retry policy used for CAS conflicts.
    policy: FastCasPolicy,
}

impl FastCas {
    /// Creates a [`FastCas`] executor that performs one CAS attempt.
    ///
    /// Equivalent to [`FastCasPolicy::once`]. After the first lost CAS,
    /// [`FastCasError::Conflict`] is returned with the observed state.
    ///
    /// # Returns
    /// A single-attempt executor.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::{FastCas, FastCasState};
    ///
    /// let cas = FastCas::once();
    /// assert_eq!(cas.policy().max_attempts(), 1);
    /// let _state = FastCasState::new(0);
    /// ```
    #[inline]
    pub const fn once() -> Self {
        Self {
            policy: FastCasPolicy::once(),
        }
    }

    /// Creates a [`FastCas`] executor that retries CAS conflicts immediately.
    ///
    /// Retries busy-spin in the current thread (no `thread::yield_now`) until
    /// success, abort, or the attempt budget is exhausted.
    ///
    /// # Parameters
    /// - `max_attempts`: Maximum number of CAS attempts. Zero is normalized to
    ///   one attempt.
    ///
    /// # Returns
    /// A spin executor.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::FastCas;
    ///
    /// let cas = FastCas::spin(64);
    /// assert_eq!(cas.policy().max_attempts(), 64);
    /// ```
    #[inline]
    pub const fn spin(max_attempts: u32) -> Self {
        Self {
            policy: FastCasPolicy::spin(max_attempts),
        }
    }

    /// Creates a [`FastCas`] executor that spins first and then yields.
    ///
    /// After `spin_attempts` attempts, [`std::thread::yield_now`] runs before
    /// subsequent retries. Caps internal spin budget to `max_attempts` when
    /// `spin_attempts` is larger.
    ///
    /// # Parameters
    /// - `spin_attempts`: Number of attempts to run before yielding.
    /// - `max_attempts`: Maximum number of CAS attempts. Zero is normalized to
    ///   one attempt.
    ///
    /// # Returns
    /// A spin-yield executor.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::FastCas;
    ///
    /// let cas = FastCas::spin_yield(8, 128);
    /// let p = cas.policy();
    /// assert_eq!(p.max_attempts(), 128);
    /// ```
    #[inline]
    pub const fn spin_yield(spin_attempts: u32, max_attempts: u32) -> Self {
        Self {
            policy: FastCasPolicy::spin_yield(spin_attempts, max_attempts),
        }
    }

    /// Creates a [`FastCas`] executor from an explicit policy.
    ///
    /// # Parameters
    /// - `policy`: Retry policy used for CAS conflicts.
    ///
    /// # Returns
    /// An executor using `policy`.
    #[inline]
    pub const fn with_policy(policy: FastCasPolicy) -> Self {
        Self { policy }
    }

    /// Returns the retry policy used by this executor.
    ///
    /// # Returns
    /// The configured retry policy.
    #[inline]
    pub const fn policy(&self) -> FastCasPolicy {
        self.policy
    }

    /// Executes a CAS operation described by a decision-returning closure.
    ///
    /// The closure receives the current state code from [`FastCasState::load`].
    /// It may run **multiple times** when another thread wins the CAS between
    /// your decision and the write; only [`FastCasDecision::Update`] performs a
    /// write. [`FastCasDecision::Finish`] succeeds without mutating `state`.
    ///
    /// # Parameters
    /// - `state`: Shared state code.
    /// - `operation`: Operation to evaluate for each observed state.
    ///
    /// # Returns
    /// A success result when the operation updates or finishes.
    ///
    /// # Errors
    /// Returns [`FastCasError::Abort`] when the operation aborts. Returns
    /// [`FastCasError::Conflict`] when CAS conflicts exhaust the retry policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::{FastCas, FastCasDecision, FastCasState};
    ///
    /// let cas = FastCas::spin(16);
    /// let state = FastCasState::new(0);
    ///
    /// let ok = cas
    ///     .execute(&state, |n| -> FastCasDecision<&str, ()> {
    ///         if n == 0 {
    ///             FastCasDecision::update(1, "first")
    ///         } else {
    ///             FastCasDecision::finish("noop")
    ///         }
    ///     })
    ///     .unwrap();
    ///
    /// assert_eq!(ok.current(), 1);
    /// assert_eq!(*ok.output(), "first");
    /// ```
    #[inline(always)]
    pub fn execute<R, E, F>(&self, state: &FastCasState, operation: F) -> Result<FastCasSuccess<R>, FastCasError<E>>
    where
        F: Fn(usize) -> FastCasDecision<R, E>,
    {
        let max_attempts = self.policy.max_attempts();
        let mut attempts = 1;
        loop {
            let current = state.load();
            match operation(current) {
                FastCasDecision::Update { next, output } => match state.compare_set(current, next) {
                    Ok(()) => {
                        return Ok(FastCasSuccess::updated(current, next, output, attempts));
                    }
                    Err(actual) if attempts >= max_attempts => {
                        return Err(FastCasError::Conflict {
                            current: actual,
                            attempts,
                        });
                    }
                    Err(_) => {}
                },
                FastCasDecision::Finish { output } => {
                    return Ok(FastCasSuccess::finished(current, output, attempts));
                }
                FastCasDecision::Abort { error } => {
                    return Err(FastCasError::Abort {
                        current,
                        error,
                        attempts,
                    });
                }
            }
            attempts += 1;
            if self.policy.should_yield_before(attempts) {
                thread::yield_now();
            }
        }
    }

    /// Executes a compact update operation.
    ///
    /// `Ok((next, output))` maps to [`FastCasDecision::Update`] and attempts to
    /// install `next`, returning `output` after the CAS succeeds.
    /// `Err(error)` maps to [`FastCasDecision::Abort`] and performs no write.
    ///
    /// This is a thin wrapper over [`Self::execute`]; the same re-entrancy and
    /// conflict-retry rules apply.
    ///
    /// # Parameters
    /// - `state`: Shared state code.
    /// - `operation`: Operation to evaluate for each observed state.
    ///
    /// # Returns
    /// A success result after `next` is installed.
    ///
    /// # Errors
    /// Returns [`FastCasError::Abort`] when `operation` returns `Err`. Returns
    /// [`FastCasError::Conflict`] when CAS conflicts exhaust the retry policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::{FastCas, FastCasState};
    ///
    /// let cas = FastCas::default();
    /// let state = FastCasState::new(2);
    ///
    /// let ok = cas
    ///     .update_by(&state, |current| Ok::<(usize, usize), ()>((current + 1, current + 1)))
    ///     .unwrap();
    ///
    /// assert_eq!(ok.current(), 3);
    /// assert_eq!(ok.into_output(), 3);
    /// ```
    #[inline(always)]
    pub fn update_by<R, E, F>(&self, state: &FastCasState, operation: F) -> Result<FastCasSuccess<R>, FastCasError<E>>
    where
        F: Fn(usize) -> Result<(usize, R), E>,
    {
        self.execute(state, |current| match operation(current) {
            Ok((next, output)) => FastCasDecision::update(next, output),
            Err(error) => FastCasDecision::abort(error),
        })
    }

    /// Attempts one fixed expected-to-next transition.
    ///
    /// Performs exactly **one** [`FastCasState::compare_set`] call. The
    /// executor's [`FastCasPolicy`] is **not** used: there is no retry loop. If
    /// the loaded value is not `expected`, or the CAS fails, this returns
    /// [`FastCasError::Conflict`] with the observed `current` state.
    ///
    /// For multi-step or observe-then-decide logic, use [`Self::execute`] or
    /// [`Self::update_by`] instead.
    ///
    /// # Parameters
    /// - `state`: Shared state code.
    /// - `expected`: Required current state code.
    /// - `next`: State code to install when `expected` matches.
    ///
    /// # Returns
    /// A success result with unit output when the transition is installed.
    ///
    /// # Errors
    /// Returns conflict if the current state is not `expected` or if the CAS
    /// write loses the race.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::{FastCas, FastCasState};
    ///
    /// let cas = FastCas::once();
    /// let state = FastCasState::new(0);
    ///
    /// let ok = cas.compare_update(&state, 0, 1).unwrap();
    /// assert!(ok.is_updated());
    /// assert_eq!(ok.current(), 1);
    /// ```
    #[inline(always)]
    pub fn compare_update(
        &self,
        state: &FastCasState,
        expected: usize,
        next: usize,
    ) -> Result<FastCasSuccess<()>, FastCasError<Infallible>> {
        self.compare_update_with(state, expected, next, |_, _| ())
    }

    /// Attempts one fixed expected-to-next transition and computes output after
    /// success.
    ///
    /// Same single-CAS semantics as [`Self::compare_update`]. The `output`
    /// closure runs **only** after a successful CAS; it receives `(expected,
    /// next)` so callers can build messages or metrics without observing stale
    /// races.
    ///
    /// # Parameters
    /// - `state`: Shared state code.
    /// - `expected`: Required current state code.
    /// - `next`: State code to install when `expected` matches.
    /// - `output`: Output factory invoked after a successful CAS write.
    ///
    /// # Returns
    /// A success result with the computed output.
    ///
    /// # Errors
    /// Returns conflict if the current state is not `expected` or if the CAS
    /// write loses the race.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_cas::{FastCas, FastCasState};
    ///
    /// let cas = FastCas::once();
    /// let state = FastCasState::new(10);
    ///
    /// let ok = cas
    ///     .compare_update_with(&state, 10, 11, |prev, next| prev + next)
    ///     .unwrap();
    ///
    /// assert_eq!(ok.into_output(), 21);
    /// ```
    #[inline(always)]
    pub fn compare_update_with<R, F>(
        &self,
        state: &FastCasState,
        expected: usize,
        next: usize,
        output: F,
    ) -> Result<FastCasSuccess<R>, FastCasError<Infallible>>
    where
        F: Fn(usize, usize) -> R,
    {
        let attempts = 1;
        match state.compare_set(expected, next) {
            Ok(()) => Ok(FastCasSuccess::updated(
                expected,
                next,
                output(expected, next),
                attempts,
            )),
            Err(current) => Err(FastCasError::Conflict { current, attempts }),
        }
    }
}

impl Default for FastCas {
    /// Creates the default fast CAS executor.
    ///
    /// Equivalent to calling [`FastCas::spin`] with `16` attempts: a small spin
    /// bound suitable for low-latency contention without yielding by default.
    ///
    /// # Returns
    /// A latency-oriented executor with a small spin budget.
    #[inline]
    fn default() -> Self {
        Self::spin(16)
    }
}
