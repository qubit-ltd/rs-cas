use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;
use qubit_cas::CasLimitKind;
use qubit_cas::CasTermination;

#[test]
fn test_abort_preserves_business_error() {
    let state = AtomicRef::from_value(1usize);
    let error = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(3)
        .no_delay()
        .build()
        .expect("valid builder")
        .execute_result(&state, |_current: &usize| CasDecision::<usize, (), &str>::abort("bad"))
        .expect_err("abort must fail");
    assert_eq!(error.kind(), CasErrorKind::Abort);
    assert_eq!(error.termination(), CasTermination::Aborted);
    assert_eq!(error.error(), Some(&"bad"));
}

#[test]
fn test_retry_exhaustion_has_domain_termination() {
    let state = AtomicRef::from_value(1usize);
    let error = CasExecutor::<usize, &'static str>::builder()
        .max_attempts(1)
        .no_delay()
        .build()
        .expect("valid builder")
        .execute_result(&state, |_current: &usize| CasDecision::<usize, (), &str>::retry("busy"))
        .expect_err("retry must fail");
    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(
        error.termination(),
        CasTermination::LimitExceeded(CasLimitKind::Attempts)
    );
}
