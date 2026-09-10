// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors returned when constructing a CAS executor.

use std::error::Error;
use std::fmt;

/// Invalid CAS builder configuration.
///
/// # Examples
///
/// ```
/// use qubit_cas::CasExecutor;
///
/// let error = CasExecutor::<usize, ()>::builder().max_attempts(0).build().unwrap_err();
/// assert!(!error.message().is_empty());
/// assert!(error.to_string().contains(error.field()));
/// ```
#[must_use = "invalid CAS configuration must be handled"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasBuildError {
    /// Boundary that rejected the configuration; diagnostic text, not a parsing
    /// key.
    pub(crate) field: &'static str,
    /// Owned validation detail supplied by the policy constructor.
    pub(crate) message: String,
}

impl CasBuildError {
    /// Retains the invalid boundary and takes ownership of validation details.
    ///
    /// # Parameters
    /// - `field`: Configuration boundary that rejected the value.
    /// - `message`: Human-readable reason; wording is not a stable protocol.
    ///
    /// # Returns
    /// An owned construction error.
    #[inline]
    pub(crate) fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    /// Returns the invalid configuration field.
    ///
    /// # Returns
    /// The static validation boundary label, intended for diagnostics.
    #[must_use]
    #[inline(always)]
    pub fn field(&self) -> &'static str {
        self.field
    }

    /// Returns a human-readable validation message.
    ///
    /// # Returns
    /// Validation details borrowed from this error; no allocation is performed.
    #[must_use]
    #[inline(always)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CasBuildError {
    /// Writes the rejected boundary and its validation detail.
    ///
    /// # Parameters
    /// - `f`: Destination formatter.
    ///
    /// # Returns
    /// Formatting completion status.
    ///
    /// # Errors
    /// Returns a formatting error if the destination rejects a write.
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid CAS builder field `{}`: {}", self.field, self.message)
    }
}

impl Error for CasBuildError {}
