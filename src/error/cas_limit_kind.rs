//! CAS retry limit categories.

/// Limit that prevented another CAS attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasLimitKind {
    /// Maximum number of attempts.
    Attempts,
    /// Cumulative operation execution time.
    OperationElapsed,
    /// Total flow elapsed time.
    TotalElapsed,
}
