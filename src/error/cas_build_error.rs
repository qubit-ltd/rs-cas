//! Errors returned when constructing a CAS executor.

use std::fmt;

/// Invalid CAS builder configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasBuildError {
    pub(crate) field: &'static str,
    pub(crate) message: String,
}

impl CasBuildError {
    pub(crate) fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    /// Returns the invalid configuration field.
    #[must_use]
    pub fn field(&self) -> &'static str {
        self.field
    }

    /// Returns a human-readable validation message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CasBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid CAS builder field `{}`: {}", self.field, self.message)
    }
}

impl std::error::Error for CasBuildError {}
