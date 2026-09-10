// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Owned diagnostic context without exposing retry implementation types.

use std::fmt;

use super::CasDiagnosticKind;

/// Execution diagnostic with a stable category and human-readable details.
///
/// Use [`Self::kind`] for classification. The message preserves the underlying
/// component, phase, callback index, and panic text when available, but its
/// wording is not a stable machine-readable protocol.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasDiagnostic;
/// use qubit_cas::CasDiagnosticKind;
///
/// let state = AtomicRef::from_value(3usize);
/// let error = CasExecutor::<usize, &'static str>::builder().build().unwrap()
///     .execute_result(&state, |_: &usize| CasDecision::<usize, (), _>::abort("sold out"))
///     .unwrap_err();
/// let diagnostic: Option<&CasDiagnostic> = error.diagnostic();
/// if let Some(detail) = diagnostic {
///     match detail.kind() {
///         CasDiagnosticKind::Timer | CasDiagnosticKind::Clock => eprintln!("timing infrastructure: {}", detail.message()),
///         _ => eprintln!("execution infrastructure: {}", detail.message()),
///     }
/// }
/// assert!(diagnostic.is_none(), "ordinary business aborts have no infrastructure cause");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasDiagnostic {
    /// Stable diagnostic category.
    kind: CasDiagnosticKind,
    /// Owned diagnostic details supplied by the retry boundary.
    message: Box<str>,
}

impl CasDiagnostic {
    /// Stores a diagnostic category and takes ownership of its message.
    ///
    /// # Parameters
    /// - `kind`: Stable classification assigned at the retry boundary.
    /// - `message`: Details consumed into owned storage.
    ///
    /// # Returns
    /// A diagnostic owning its message independently of upstream error
    /// lifetimes.
    #[must_use]
    #[inline]
    pub(crate) fn new(kind: CasDiagnosticKind, message: impl Into<Box<str>>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns the stable category of this diagnostic.
    ///
    /// # Returns
    /// The stable classification independent of the retained message wording.
    #[must_use]
    #[inline(always)]
    pub fn kind(&self) -> CasDiagnosticKind {
        self.kind
    }

    /// Returns retained diagnostic details; the wording may change between
    /// versions.
    ///
    /// # Returns
    /// Diagnostic text borrowed for the lifetime of this value.
    #[must_use]
    #[inline(always)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CasDiagnostic {
    /// Writes the retained diagnostic details to the formatter.
    ///
    /// # Parameters
    /// - `f`: Destination formatter.
    ///
    /// # Returns
    /// Formatting completion status.
    ///
    /// # Errors
    /// Returns a formatting error if the destination rejects a write.
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
