#![cfg(feature = "tokio")]

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasError;
use qubit_cas::CasExecutor;
use qubit_cas::CasRetryFailure;
use qubit_cas::CasSuccess;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryTimeoutScope;

async fn run_case<F, Fut>(
    executor: &CasExecutor<usize, &'static str>,
    state: &AtomicRef<usize>,
    rich: bool,
    operation: F,
) -> Result<CasSuccess<usize, ()>, CasError<usize, &'static str>>
where
    F: Fn(Arc<usize>) -> Fut,
    Fut: Future<Output = CasDecision<usize, (), &'static str>>,
{
    if rich {
        executor.execute_async(state, operation).await.into_result()
    } else {
        executor.execute_async_result(state, operation).await
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_soft_budget_keeps_admitted_success() {
    for rich in [false, true] {
        let state = AtomicRef::from_value(5usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .max_total_elapsed(Some(Duration::from_secs(1)))
            .build()
            .expect("executor");
        let success = run_case(&executor, &state, rich, |_| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            CasDecision::finish(())
        })
        .await
        .expect("soft budget must preserve admitted success");
        assert_eq!(success.attempts(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_soft_budget_rejects_next_attempt() {
    for rich in [false, true] {
        let state = AtomicRef::from_value(5usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .max_attempts(3)
            .max_total_elapsed(Some(Duration::from_secs(1)))
            .build()
            .expect("executor");
        let calls = Arc::new(AtomicUsize::new(0));
        let recorded = Arc::clone(&calls);
        let error = run_case(&executor, &state, rich, move |_| {
            recorded.fetch_add(1, Ordering::SeqCst);
            async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                CasDecision::retry("busy")
            }
        })
        .await
        .expect_err("continuation rejected");
        assert!(matches!(
            error.failure(),
            CasRetryFailure::Exhausted {
                limit: RetryLimitKind::TotalElapsed
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(error.attempts(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_explicit_flow_timeout() {
    for rich in [false, true] {
        let state = AtomicRef::from_value(5usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .max_attempts(3)
            .flow_timeout(Some(Duration::from_secs(1)))
            .build()
            .expect("executor");
        let error = run_case(&executor, &state, rich, |_| {
            std::future::pending::<CasDecision<usize, (), &'static str>>()
        })
        .await
        .expect_err("hard flow timeout");
        assert!(matches!(
            error.failure(),
            CasRetryFailure::TimedOut {
                scope: RetryTimeoutScope::Flow
            }
        ));
        assert_eq!(error.attempts(), 1);
        assert!(error.current().is_some());
        assert_eq!(executor.policy().limits().max_total_elapsed(), None);
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_zero_flow_never_calls_operation() {
    for rich in [false, true] {
        let state = AtomicRef::from_value(5usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .flow_timeout(Some(Duration::ZERO))
            .build()
            .expect("executor");
        let error = run_case(&executor, &state, rich, |_| {
            panic!("operation must not be constructed");
            #[allow(unreachable_code)]
            std::future::ready(CasDecision::finish(()))
        })
        .await
        .expect_err("zero timeout");
        assert_eq!(error.attempts(), 0);
        assert!(matches!(
            error.failure(),
            CasRetryFailure::TimedOut {
                scope: RetryTimeoutScope::Flow
            }
        ));
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_flow_timeout_bounds_backoff() {
    for rich in [false, true] {
        let state = AtomicRef::from_value(5usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .max_attempts(3)
            .fixed_delay(Duration::from_secs(20))
            .flow_timeout(Some(Duration::from_secs(1)))
            .build()
            .expect("executor");
        let error = run_case(&executor, &state, rich, |_| async { CasDecision::retry("busy") })
            .await
            .expect_err("flow deadline interrupts backoff");
        assert!(matches!(
            error.failure(),
            CasRetryFailure::TimedOut {
                scope: RetryTimeoutScope::Flow
            }
        ));
        assert_eq!(error.attempts(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_combined_limits_stay_independent() {
    for rich in [false, true] {
        let state = AtomicRef::from_value(5usize);
        let executor = CasExecutor::<usize, &'static str>::builder()
            .max_total_elapsed(Some(Duration::from_secs(1)))
            .flow_timeout(Some(Duration::from_secs(3)))
            .build()
            .expect("executor");
        let success = run_case(&executor, &state, rich, |_| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            CasDecision::finish(())
        })
        .await
        .expect("soft limit must not shorten the explicit hard limit");
        assert_eq!(success.attempts(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn cas_timeout_contract_equal_timeout_prefers_attempt_scope() {
    let state = AtomicRef::from_value(5usize);
    let executor = CasExecutor::<usize, &'static str>::builder()
        .attempt_timeout(Some(Duration::from_secs(1)))
        .flow_timeout(Some(Duration::from_secs(1)))
        .build()
        .expect("executor");
    let error = executor
        .execute_async_result(&state, |_| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            CasDecision::finish(())
        })
        .await
        .expect_err("equal timeout should terminate");
    assert!(matches!(
        error.failure(),
        CasRetryFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt
        }
    ));
}
