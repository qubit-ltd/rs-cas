//! Listener locations used by CAS diagnostics.

/// Location at which an observation listener failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CasListenerKind {
    /// Execution-started event listener.
    ExecutionStarted,
    /// Attempt-failed event listener.
    AttemptFailed,
    /// Retry-scheduled event listener.
    RetryScheduled,
    /// Execution-finished event listener.
    ExecutionFinished,
    /// Contention alert listener.
    ContentionAlert,
}
