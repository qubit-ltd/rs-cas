// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! CAS decision values returned by user operations.

use std::sync::Arc;

/// Decision returned by one CAS operation attempt.
///
/// A CAS operation reads a consistent snapshot of `T` and returns one of four
/// outcomes:
///
/// - [`Update`](Self::Update): install a new state by compare-and-swap.
/// - [`Finish`](Self::Finish): treat the attempt as successful without writing.
/// - [`Retry`](Self::Retry): report a retryable business failure.
/// - [`Abort`](Self::Abort): report a terminal business failure.
///
/// ```compile_fail
/// #![deny(unused_must_use)]
///
/// use qubit_cas::CasDecision;
///
/// CasDecision::<usize, (), ()>::finish(());
/// ```
///
/// # Type Parameters
/// - `T`: Shared application state.
/// - `R`: Output returned only from a successful attempt.
/// - `E`: Business failure used for Retry or Abort.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
///
/// let state = AtomicRef::from_value(3usize);
/// let executor = CasExecutor::<usize, ()>::builder().build().unwrap();
/// let success = executor.execute_result(&state, |current: &usize| {
///     CasDecision::update(*current - 1, "reserved")
/// }).unwrap();
/// assert_eq!(*state.load(), 2);
/// assert_eq!(*success.output(), "reserved");
/// ```
#[must_use = "a CAS decision must be returned to the executor"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CasDecision<T, R, E> {
    /// Writes a new state and returns a business output.
    Update {
        /// Replacement state to install when compare-and-swap succeeds.
        next: Arc<T>,
        /// Business output returned to the caller on success.
        output: R,
    },

    /// Completes successfully without writing a new state.
    Finish {
        /// Business output returned to the caller.
        output: R,
    },

    /// Requests another CAS attempt.
    Retry(
        /// Business failure retained if the flow cannot continue.
        E,
    ),

    /// Aborts the CAS flow immediately.
    Abort(
        /// Business failure explaining the terminal abort.
        E,
    ),
}

impl<T, R, E> CasDecision<T, R, E> {
    /// Creates an update decision from an owned value.
    ///
    /// # Parameters
    /// - `next`: Replacement state to wrap in [`Arc`] and install.
    /// - `output`: Business output returned on success.
    ///
    /// # Returns
    /// A [`CasDecision::Update`] value.
    #[inline]
    pub fn update(next: T, output: R) -> Self {
        Self::Update {
            next: Arc::new(next),
            output,
        }
    }

    /// Creates an update decision from an `Arc<T>`.
    ///
    /// # Parameters
    /// - `next`: Replacement state to install.
    /// - `output`: Business output returned on success.
    ///
    /// # Returns
    /// A [`CasDecision::Update`] value.
    #[inline(always)]
    pub fn update_arc(next: Arc<T>, output: R) -> Self {
        Self::Update { next, output }
    }

    /// Creates a finish decision.
    ///
    /// # Parameters
    /// - `output`: Business output returned to the caller.
    ///
    /// # Returns
    /// A [`CasDecision::Finish`] value.
    #[inline(always)]
    pub fn finish(output: R) -> Self {
        Self::Finish { output }
    }

    /// Creates a retry decision.
    ///
    /// # Parameters
    /// - `error`: Retryable business failure.
    ///
    /// # Returns
    /// A [`CasDecision::Retry`] value.
    #[inline(always)]
    pub fn retry(error: E) -> Self {
        Self::Retry(error)
    }

    /// Creates an abort decision.
    ///
    /// # Parameters
    /// - `error`: Terminal business failure.
    ///
    /// # Returns
    /// A [`CasDecision::Abort`] value.
    #[inline(always)]
    pub fn abort(error: E) -> Self {
        Self::Abort(error)
    }
}
