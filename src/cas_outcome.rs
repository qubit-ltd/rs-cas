// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Combined CAS result and execution report.

use std::fmt;

use crate::cas_success::CasSuccess;
use crate::error::CasError;
use crate::report::CasExecutionReport;

/// Public return value for one CAS execution.
///
/// Combines the terminal [`Result`] (`CasSuccess` or [`CasError`]) with the
/// full [`CasExecutionReport`] for observability.
#[derive(Debug, Clone)]
pub struct CasOutcome<T, R, E> {
    /// Terminal success or error.
    result: Result<CasSuccess<T, R>, CasError<T, E>>,
    /// Execution report captured for the flow.
    report: CasExecutionReport,
}

impl<T, R, E> CasOutcome<T, R, E> {
    /// Creates a CAS outcome from a terminal result and execution report.
    ///
    /// # Parameters
    /// - `result`: The terminal [`Result`] from the CAS operation.
    /// - `report`: The full execution report with statistics and timing.
    ///
    /// # Returns
    /// A new [`CasOutcome`] wrapping both values.
    #[inline]
    pub(crate) fn new(
        result: Result<CasSuccess<T, R>, CasError<T, E>>,
        report: CasExecutionReport,
    ) -> Self {
        Self { result, report }
    }

    /// Returns the terminal result.
    ///
    /// # Returns
    /// Reference to the inner [`Result<CasSuccess<T, R>, CasError<T, E>>`].
    #[inline]
    pub fn result(&self) -> &Result<CasSuccess<T, R>, CasError<T, E>> {
        &self.result
    }

    /// Returns the execution report.
    ///
    /// # Returns
    /// Reference to the [`CasExecutionReport`] captured during execution.
    #[inline]
    pub fn report(&self) -> &CasExecutionReport {
        &self.report
    }

    /// Returns whether the execution succeeded.
    ///
    /// # Returns
    /// `true` if the terminal result is [`Ok`].
    #[inline]
    pub fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    /// Returns whether the execution failed.
    ///
    /// # Returns
    /// `true` if the terminal result is [`Err`].
    #[inline]
    pub fn is_err(&self) -> bool {
        self.result.is_err()
    }

    /// Consumes the outcome and returns its terminal result.
    ///
    /// # Returns
    /// The owned [`Result<CasSuccess<T, R>, CasError<T, E>>`].
    #[inline]
    pub fn into_result(self) -> Result<CasSuccess<T, R>, CasError<T, E>> {
        self.result
    }

    /// Consumes the outcome and returns its parts.
    ///
    /// # Returns
    /// Tuple of the terminal result and the execution report.
    #[inline]
    pub fn into_parts(
        self,
    ) -> (Result<CasSuccess<T, R>, CasError<T, E>>, CasExecutionReport) {
        (self.result, self.report)
    }

    /// Returns the success value or panics with the supplied message.
    ///
    /// This mirrors [`Result::expect`] for call sites that only need the
    /// success value during tests or examples.
    ///
    /// # Parameters
    /// - `message`: Panic message used if the outcome is an error.
    ///
    /// # Returns
    /// The inner [`CasSuccess<T, R>`] on success.
    ///
    /// # Panics
    /// Panics with the given message if the outcome contains an error.
    #[inline]
    pub fn expect(self, message: &str) -> CasSuccess<T, R>
    where
        E: fmt::Debug,
    {
        self.result.expect(message)
    }

    /// Returns the error value or panics with the supplied message.
    ///
    /// This mirrors [`Result::expect_err`] for call sites that only need the
    /// terminal error during tests or examples.
    ///
    /// # Parameters
    /// - `message`: Panic message used if the outcome is a success.
    ///
    /// # Returns
    /// The inner [`CasError<T, E>`] on error.
    ///
    /// # Panics
    /// Panics with the given message if the outcome is successful.
    #[inline]
    pub fn expect_err(self, message: &str) -> CasError<T, E>
    where
        T: fmt::Debug,
        R: fmt::Debug,
    {
        self.result.expect_err(message)
    }
}
