//! Private storage for terminal CAS diagnostics.

use crate::error::CasTermination;
use crate::event::CasContext;

/// Compact, non-generic details shared by a CAS error.
#[derive(Debug, Clone)]
pub(crate) struct CasErrorDetails {
    pub(crate) termination: CasTermination,
    pub(crate) context: CasContext,
}
