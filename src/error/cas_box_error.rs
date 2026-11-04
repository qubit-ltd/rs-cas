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
    ///
    /// # Parameters
    ///
    /// - `inner`: The boxed business error to retain without reallocation.
    ///
    /// # Returns
    ///
    /// A wrapper owning the supplied error.
    #[must_use]
    #[inline]
    pub fn new(inner: BoxError) -> Self {
        Self { inner }
    }

    /// Borrows the original error, including its thread-safety bounds.
    ///
    /// # Returns
    ///
    /// A reference valid for the duration of the borrow of this wrapper.
    #[must_use]
    #[inline]
    pub fn as_inner(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.inner.as_ref()
    }

    /// Consumes the wrapper and returns ownership of the original boxed error.
    ///
    /// # Returns
    ///
    /// The original box, moved out without reallocating or cloning the error.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> BoxError {
        self.inner
    }
}

impl From<BoxError> for CasBoxError {
    /// Wraps an already boxed business error.
    ///
    /// # Parameters
    ///
    /// - `inner`: The boxed business error whose ownership is transferred.
    ///
    /// # Returns
    ///
    /// A wrapper retaining the supplied box as its source.
    #[inline]
    fn from(inner: BoxError) -> Self {
        Self::new(inner)
    }
}

impl fmt::Display for CasBoxError {
    /// Delegates display formatting to the original error.
    ///
    /// # Parameters
    ///
    /// - `f`: The destination and formatting options for the original error.
    ///
    /// # Errors
    ///
    /// Returns the original error's formatting failure, if any.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl fmt::Debug for CasBoxError {
    /// Delegates debug formatting to the original error.
    ///
    /// # Parameters
    ///
    /// - `f`: The destination and debug formatting options for the original
    ///   error.
    ///
    /// # Errors
    ///
    /// Returns the original error's formatting failure, if any.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl Error for CasBoxError {
    /// Always returns the original boxed business error as the source.
    ///
    /// # Returns
    ///
    /// Always `Some` containing the original error borrowed from this wrapper;
    /// never `None`, because every wrapper owns a source error.
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.as_inner())
    }
}
