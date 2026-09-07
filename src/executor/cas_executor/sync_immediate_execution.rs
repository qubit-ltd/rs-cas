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
    // Keep immediate execution behind a dedicated dispatch point so its
    // allocation and loop strategy can evolve without changing the public API.
    executor.execute_result_generic(state, operation)
}
