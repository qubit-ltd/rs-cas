// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stable categories for retry diagnostics exposed by CAS.

/// Category of an execution diagnostic, independent of retry implementation
/// types.
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
/// assert_eq!(error.error(), Some(&"sold out"));
/// assert!(diagnostic.is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CasDiagnosticKind {
    /// Cancellation was observed by the retry infrastructure.
    Cancellation,
    /// A retry control or completion callback failed.
    Callback,
    /// The monotonic clock could not provide a valid reading.
    Clock,
    /// A retry timer failed.
    Timer,
    /// Another retry infrastructure component failed.
    Infrastructure,
    /// A newer retry version supplied an unrecognized terminal reason.
    UnknownRetryReason,
}
