/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Successful fast compare-and-swap results.

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FastCasSuccessKind {
    Updated,
    Finished,
}

/// Successful [`crate::FastCas`] result.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FastCasSuccess<R> {
    /// State observed by the successful attempt.
    previous: usize,
    /// State after completion.
    current: usize,
    /// Business output.
    output: R,
    /// Number of attempts consumed.
    attempts: u32,
    /// Decision path that produced the success.
    kind: FastCasSuccessKind,
}

impl<R> FastCasSuccess<R> {
    /// Creates a successful update result.
    ///
    /// # Parameters
    /// - `previous`: State observed by the successful attempt.
    /// - `current`: State after completion.
    /// - `output`: Business output.
    /// - `attempts`: Number of attempts consumed.
    ///
    /// # Returns
    /// A successful update result value.
    #[inline]
    pub(crate) const fn updated(previous: usize, current: usize, output: R, attempts: u32) -> Self {
        Self {
            previous,
            current,
            output,
            attempts,
            kind: FastCasSuccessKind::Updated,
        }
    }

    /// Creates a successful finish result.
    ///
    /// # Parameters
    /// - `current`: State observed when the operation finished.
    /// - `output`: Business output.
    /// - `attempts`: Number of attempts consumed.
    ///
    /// # Returns
    /// A successful finish result value.
    #[inline]
    pub(crate) const fn finished(current: usize, output: R, attempts: u32) -> Self {
        Self {
            previous: current,
            current,
            output,
            attempts,
            kind: FastCasSuccessKind::Finished,
        }
    }

    /// Returns the state observed before successful completion.
    ///
    /// # Returns
    /// The state code read by the successful attempt.
    #[inline]
    pub const fn previous(&self) -> usize {
        self.previous
    }

    /// Returns the state after completion.
    ///
    /// # Returns
    /// The installed state for updates, or [`previous`](Self::previous) for
    /// finishes.
    #[inline]
    pub const fn current(&self) -> usize {
        self.current
    }

    /// Returns a shared reference to the business output.
    ///
    /// # Returns
    /// The output produced by the operation.
    #[inline]
    pub const fn output(&self) -> &R {
        &self.output
    }

    /// Consumes this result and returns the business output.
    ///
    /// # Returns
    /// The output produced by the operation.
    #[inline]
    pub fn into_output(self) -> R {
        self.output
    }

    /// Returns the number of attempts consumed.
    ///
    /// # Returns
    /// Number of attempts executed by the operation.
    #[inline]
    pub const fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Tests whether this success came from an update decision.
    ///
    /// # Returns
    /// `true` when the operation successfully executed an update decision,
    /// even if the installed state equals the previously observed state.
    #[inline]
    pub const fn is_updated(&self) -> bool {
        matches!(self.kind, FastCasSuccessKind::Updated)
    }

    /// Tests whether this success came from a finish decision.
    ///
    /// # Returns
    /// `true` when the operation completed through a finish decision.
    #[inline]
    pub const fn is_finished(&self) -> bool {
        matches!(self.kind, FastCasSuccessKind::Finished)
    }
}
