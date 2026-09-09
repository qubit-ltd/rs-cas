// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Owned diagnostic context without exposing retry implementation types.

use std::fmt;

use super::CasDiagnosticKind;

/// Execution diagnostic with a stable category and human-readable details.
///
/// Use [`Self::kind`] for classification. The message preserves the underlying
/// component, phase, callback index, and panic text when available, but its
/// wording is not a stable machine-readable protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasDiagnostic {
    /// Stable diagnostic category.
    kind: CasDiagnosticKind,
    /// Owned diagnostic details supplied by the retry boundary.
    message: Box<str>,
}

impl CasDiagnostic {
    /// Stores a diagnostic category and takes ownership of its message.
    pub(crate) fn new(kind: CasDiagnosticKind, message: impl Into<Box<str>>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns the stable category of this diagnostic.
    #[must_use]
    pub fn kind(&self) -> CasDiagnosticKind {
        self.kind
    }

    /// Returns retained diagnostic details; the wording may change between
    /// versions.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CasDiagnostic {
    /// Writes the retained diagnostic details to the formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
