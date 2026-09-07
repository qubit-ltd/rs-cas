//! CAS timeout scopes.

/// Scope of a timeout that terminated CAS execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasTimeoutScope {
    /// One operation attempt.
    Attempt,
    /// The whole retry flow.
    Flow,
}
