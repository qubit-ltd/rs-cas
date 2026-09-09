#[cfg(feature = "tokio")]
use std::time::Duration;

#[cfg(feature = "tokio")]
use qubit_atomic::AtomicRef;
#[cfg(feature = "tokio")]
use qubit_cas::CasDecision;
#[cfg(feature = "tokio")]
use qubit_cas::CasErrorKind;
#[cfg(feature = "tokio")]
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;

#[test]
fn test_builder_returns_cas_build_error() {
    let error = CasExecutor::<usize, ()>::builder()
        .max_attempts(0)
        .build()
        .expect_err("zero attempts must be rejected");
    assert_eq!(error.field(), "retry_policy");
    assert!(error.message().contains("attempt"));
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn test_flow_timeout_is_hard_and_distinct_from_total_budget() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .flow_timeout(Some(Duration::from_millis(1)))
        .no_delay()
        .build()
        .expect("valid builder");
    let outcome = executor
        .execute_async_with_hooks(
            &state,
            |_current: std::sync::Arc<usize>| async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                CasDecision::<usize, (), ()>::retry(())
            },
            Default::default(),
        )
        .await;
    let error = outcome.result().as_ref().expect_err("retry must terminate");
    assert_eq!(error.kind(), CasErrorKind::FlowTimeout);
    assert_ne!(
        outcome.report().outcome(),
        CasExecutionOutcome::ErrorTotalBudgetExceeded
    );
}
