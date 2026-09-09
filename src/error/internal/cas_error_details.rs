//! Private storage for terminal CAS diagnostics.

use crate::error::CasDiagnostic;
use crate::error::CasTermination;
use crate::event::CasContext;

/// Compact, non-generic details shared by a CAS error.
#[derive(Debug, Clone)]
pub(crate) struct CasErrorDetails {
    /// Structured terminal reason.
    pub(crate) termination: CasTermination,
    /// Terminal retry context.
    pub(crate) context: CasContext,
    /// Infrastructure cause, when the terminal reason carries one.
    pub(crate) diagnostic: Option<CasDiagnostic>,
    /// Failures from callbacks invoked after the terminal result was frozen.
    pub(crate) completion_diagnostics: Box<[CasDiagnostic]>,
}
