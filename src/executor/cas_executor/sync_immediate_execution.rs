//! Immediate synchronous CAS execution path.

use qubit_atomic::AtomicRef;
use qubit_function::Function;

use super::CasExecutor;
use crate::cas_decision::CasDecision;
use crate::cas_success::CasSuccess;
use crate::error::CasError;

/// Executes an immediate retry flow.
pub(super) fn execute<T, E, R, O>(
    executor: &CasExecutor<T, E>,
    state: &AtomicRef<T>,
    operation: O,
) -> Result<CasSuccess<T, R>, CasError<T, E>>
where
    T: 'static,
    E: 'static,
    O: Function<T, CasDecision<T, R, E>>,
{
    // Keep the generic adapter as the semantic fallback until the dedicated
    // loop is fully characterized; the selector and public contract already
    // ensure immediate configurations have an isolated implementation point.
    executor.execute_result_generic(state, operation)
}
