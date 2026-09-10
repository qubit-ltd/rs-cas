// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Standard-error wrapper for the default CAS business error.

use std::error::Error;
use std::fmt;

use qubit_error::BoxError;

/// Owned, type-erased business error used by default CAS executors and
/// builders.
///
/// The wrapper implements [`Error`] and preserves the original error as its
/// source. Concrete errors must be boxed explicitly; there is no blanket
/// `From<E>` conversion because it would overlap with `From<T> for T`.
///
/// # Examples
///
/// ```
/// use std::error::Error;
/// use qubit_cas::CasBoxError;
///
/// let error = CasBoxError::new(Box::new(std::io::Error::other("reservation failed")));
/// assert_eq!(error.source().unwrap().to_string(), "reservation failed");
/// ```
pub struct CasBoxError {
    /// Original business error, retained without changing its concrete type.
    inner: BoxError,
}

impl CasBoxError {
    /// Takes ownership of `inner`, retaining it as this wrapper's source.
    #[must_use]
    #[inline]
    pub fn new(inner: BoxError) -> Self {
        Self { inner }
    }

    /// Borrows the original error, including its thread-safety bounds.
    #[must_use]
    #[inline(always)]
    pub fn as_inner(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.inner.as_ref()
    }

    /// Consumes the wrapper and returns ownership of the original boxed error.
    #[must_use]
    #[inline(always)]
    pub fn into_inner(self) -> BoxError {
        self.inner
    }
}

impl From<BoxError> for CasBoxError {
    /// Wraps an already boxed business error.
    #[inline]
    fn from(inner: BoxError) -> Self {
        Self::new(inner)
    }
}

impl fmt::Display for CasBoxError {
    /// Delegates display formatting to the original error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl fmt::Debug for CasBoxError {
    /// Delegates debug formatting to the original error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl Error for CasBoxError {
    /// Always returns the original boxed business error as the source.
    #[inline(always)]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.as_inner())
    }
}
