// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Listener panic diagnostics.

use std::any::Any;
use std::fmt;

use super::CasListenerKind;

/// A listener panic captured without changing CAS business outcome.
///
/// # Examples
///
/// ```
/// use qubit_atomic::AtomicRef;
/// use qubit_cas::CasDecision;
/// use qubit_cas::CasExecutor;
/// use qubit_cas::CasEvent;
/// use qubit_cas::CasHooks;
/// use qubit_cas::CasListenerKind;
///
/// // Listener isolation applies to unwind builds.
/// let hooks = CasHooks::new().on_event(|event: &CasEvent| {
///     if matches!(event, CasEvent::ExecutionStarted { .. }) {
///         panic!("metrics unavailable");
///     }
/// });
/// let state = AtomicRef::from_value(3usize);
/// let outcome = CasExecutor::<usize, ()>::builder().build().unwrap()
///     .execute_with_hooks(&state, |_: &usize| CasDecision::finish(()), hooks);
/// assert!(outcome.is_ok());
/// let failure = &outcome.report().listener_failures()[0];
/// assert_eq!(failure.kind(), CasListenerKind::ExecutionStarted);
/// assert_eq!(failure.message(), "metrics unavailable");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasListenerFailure {
    /// Lifecycle stage at which the listener panicked.
    kind: CasListenerKind,
    /// Owned string payload, or a fallback for non-string panic payloads.
    message: String,
}

impl CasListenerFailure {
    /// Takes ownership of an unwind payload and retains a printable
    /// description.
    ///
    /// # Parameters
    /// - `kind`: Stage of the isolated listener invocation.
    /// - `payload`: Owned panic payload; non-string values use a fixed
    ///   fallback.
    ///
    /// # Returns
    /// A diagnostic independent of the business result.
    #[must_use]
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
    ///
    /// # Returns
    /// The stage of the failed listener invocation.
    #[must_use]
    #[inline(always)]
    pub fn kind(&self) -> CasListenerKind {
        self.kind
    }

    /// Returns the panic message.
    ///
    /// # Returns
    /// Borrowed panic text, with a fallback for non-string payloads.
    #[must_use]
    #[inline(always)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CasListenerFailure {
    /// Writes the listener stage and retained panic text.
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
        write!(f, "CAS {:?} listener failed: {}", self.kind, self.message)
    }
}
