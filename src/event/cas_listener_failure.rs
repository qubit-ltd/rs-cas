//! Listener panic diagnostics.

use std::any::Any;
use std::fmt;

use super::CasListenerKind;

/// A listener panic captured without changing CAS business outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasListenerFailure {
    kind: CasListenerKind,
    message: String,
}

impl CasListenerFailure {
    pub(crate) fn from_panic(kind: CasListenerKind, payload: Box<dyn Any + Send>) -> Self {
        let message = payload
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload")
            .to_owned();
        Self { kind, message }
    }

    /// Returns the listener location.
    #[must_use]
    pub fn kind(&self) -> CasListenerKind {
        self.kind
    }

    /// Returns the panic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CasListenerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CAS {:?} listener failed: {}", self.kind, self.message)
    }
}
