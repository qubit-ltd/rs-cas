//! CAS terminal classifications.

use super::CasLimitKind;
use super::CasTimeoutScope;

/// Structured reason why a CAS execution stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasTermination {
    /// The operation explicitly aborted.
    Aborted,
    /// A retry limit prevented another attempt.
    LimitExceeded(CasLimitKind),
    /// A hard timeout stopped execution.
    TimedOut(CasTimeoutScope),
    /// The retry infrastructure could not continue safely.
    RetryInfrastructure,
}
