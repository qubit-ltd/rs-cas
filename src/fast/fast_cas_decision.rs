// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Decision values returned by fast compare-and-swap operations.

/// Decision returned by one [`crate::FastCas`] operation attempt.
///
/// The operation may be called multiple times when CAS conflicts happen, so it
/// should be deterministic and free of non-idempotent side effects.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FastCasDecision<R, E> {
    /// Replace the current state with `next` and return `output` after the CAS
    /// write succeeds.
    Update {
        /// State code to install.
        next: usize,

        /// Business output returned after success.
        output: R,
    },

    /// Complete successfully without writing a new state.
    Finish {
        /// Business output returned immediately.
        output: R,
    },

    /// Stop immediately without writing a new state.
    Abort {
        /// Business error returned to the caller.
        error: E,
    },
}

impl<R, E> FastCasDecision<R, E> {
    /// Creates an update decision.
    ///
    /// # Parameters
    /// - `next`: State code to install.
    /// - `output`: Business output returned after the CAS succeeds.
    ///
    /// # Returns
    /// An update decision.
    #[inline]
    pub const fn update(next: usize, output: R) -> Self {
        Self::Update { next, output }
    }

    /// Creates a finish decision.
    ///
    /// # Parameters
    /// - `output`: Business output returned without writing.
    ///
    /// # Returns
    /// A finish decision.
    #[inline]
    pub const fn finish(output: R) -> Self {
        Self::Finish { output }
    }

    /// Creates an abort decision.
    ///
    /// # Parameters
    /// - `error`: Business error returned to the caller.
    ///
    /// # Returns
    /// An abort decision.
    #[inline]
    pub const fn abort(error: E) -> Self {
        Self::Abort { error }
    }
}
