//! Infrastructure diagnostics must survive retry-to-CAS projection.

#[test]
fn test_clock_failure_retains_message() {
    let error = crate::CasError::<usize, ()>::from_retry_parts(
        RetryErrorReason::Infrastructure {
            failure: RetryInfrastructureFailure::Clock {
                message: "clock moved backwards".into(),
            },
        },
        None,
        RetryContext::new(0, 3),
        None,
        Vec::new().into_boxed_slice(),
    );
    assert_eq!(error.kind(), crate::CasErrorKind::RetryInfrastructure);
    let diagnostic = error.diagnostic().expect("clock diagnostic retained");
    assert_eq!(diagnostic.kind(), CasDiagnosticKind::Clock);
    assert!(diagnostic.message().contains("clock moved backwards"));
}

use std::sync::Arc;

use qubit_retry::AttemptFailure;
use qubit_retry::RetryCallbackFailure;
use qubit_retry::RetryCallbackKind;
use qubit_retry::RetryCallbackPhase;
use qubit_retry::RetryCancellationPhase;
use qubit_retry::RetryContext;
use qubit_retry::RetryErrorReason;
use qubit_retry::RetryInfrastructureFailure;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryPanic;
use qubit_retry::RetryTimeoutScope;

use crate::CasAttemptFailure;
use crate::CasDiagnosticKind;
use crate::CasError;
use crate::CasErrorKind;
use crate::CasLimitKind;
use crate::CasTermination;
use crate::CasTimeoutScope;

/// Projects a terminal reason with an independent retained business failure.
fn project(reason: RetryErrorReason) -> CasError<usize, &'static str> {
    CasError::from_retry_parts(
        reason,
        Some(AttemptFailure::Error(CasAttemptFailure::Retry {
            current: Arc::new(4),
            error: "business",
        })),
        RetryContext::new(2, 3),
        None,
        Vec::new().into_boxed_slice(),
    )
}

#[test]
fn test_terminal_reason_precedes_last_business_error() {
    for (reason, termination, kind) in [
        (RetryErrorReason::Aborted, CasTermination::Aborted, CasErrorKind::Abort),
        (
            RetryErrorReason::Exhausted {
                limit: RetryLimitKind::Attempts,
            },
            CasTermination::LimitExceeded(CasLimitKind::Attempts),
            CasErrorKind::RetryExhausted,
        ),
        (
            RetryErrorReason::Exhausted {
                limit: RetryLimitKind::OperationElapsed,
            },
            CasTermination::LimitExceeded(CasLimitKind::OperationElapsed),
            CasErrorKind::OperationBudgetExceeded,
        ),
        (
            RetryErrorReason::Exhausted {
                limit: RetryLimitKind::TotalElapsed,
            },
            CasTermination::LimitExceeded(CasLimitKind::TotalElapsed),
            CasErrorKind::TotalBudgetExceeded,
        ),
        (
            RetryErrorReason::TimedOut {
                scope: RetryTimeoutScope::Attempt,
            },
            CasTermination::TimedOut(CasTimeoutScope::Attempt),
            CasErrorKind::AttemptTimeout,
        ),
        (
            RetryErrorReason::TimedOut {
                scope: RetryTimeoutScope::Flow,
            },
            CasTermination::TimedOut(CasTimeoutScope::Flow),
            CasErrorKind::FlowTimeout,
        ),
    ] {
        let error = project(reason);
        assert_eq!(error.kind(), kind);
        assert_eq!(error.termination(), termination);
        assert_eq!(error.error(), Some(&"business"));
        assert_eq!(**error.current().expect("retained snapshot"), 4);
        assert!(error.diagnostic().is_none());
        assert!(error.completion_diagnostics().is_empty());
    }
}

#[test]
fn test_cancellation_phases_and_timer_messages_survive_projection() {
    for phase in [
        RetryCancellationPhase::BeforeAttempt,
        RetryCancellationPhase::Attempt,
        RetryCancellationPhase::Backoff,
    ] {
        let expected = phase.to_string();
        let error = project(RetryErrorReason::Cancelled { phase });
        let diagnostic = error.diagnostic().expect("cancellation diagnostic");
        assert_eq!(diagnostic.kind(), CasDiagnosticKind::Cancellation);
        assert!(diagnostic.message().contains(&expected));
        assert_eq!(diagnostic.to_string(), diagnostic.message());
        assert!(error.to_string().contains(diagnostic.message()));
        assert_eq!(error.termination(), CasTermination::RetryInfrastructure);
    }
    let error = project(RetryErrorReason::Infrastructure {
        failure: RetryInfrastructureFailure::Timer {
            message: "timer unavailable".into(),
        },
    });
    let diagnostic = error.diagnostic().expect("timer diagnostic");
    assert_eq!(diagnostic.kind(), CasDiagnosticKind::Timer);
    assert!(diagnostic.message().contains("timer unavailable"));
    assert!(format!("{error:?}").contains("timer unavailable"));
}

#[test]
fn test_callback_attribution_and_completion_order_are_retained() {
    for category in [RetryCallbackKind::Rule, RetryCallbackKind::Observer] {
        for phase in [
            RetryCallbackPhase::BeforeAttempt,
            RetryCallbackPhase::AttemptFailed,
            RetryCallbackPhase::RuleDecision,
            RetryCallbackPhase::RetryScheduled,
            RetryCallbackPhase::Success,
            RetryCallbackPhase::TerminalFailure,
        ] {
            let failure = RetryCallbackFailure::new(category, 2, phase, RetryPanic::StaticStr("bad callback"));
            let expected = failure.to_string();
            let error = project(RetryErrorReason::CallbackFailed { callback: failure });
            let diagnostic = error.diagnostic().expect("callback diagnostic");
            assert_eq!(diagnostic.kind(), CasDiagnosticKind::Callback);
            assert!(diagnostic.message().contains(&expected));
        }
    }
    let failures = vec![
        RetryCallbackFailure::new(
            RetryCallbackKind::Observer,
            1,
            RetryCallbackPhase::TerminalFailure,
            RetryPanic::String("first callback".into()),
        ),
        RetryCallbackFailure::new(
            RetryCallbackKind::Observer,
            3,
            RetryCallbackPhase::TerminalFailure,
            RetryPanic::NonString,
        ),
    ];
    let expected = failures.iter().map(ToString::to_string).collect::<Vec<_>>();
    let error = CasError::<usize, &'static str>::from_retry_parts(
        RetryErrorReason::Aborted,
        None,
        RetryContext::new(0, 3),
        None,
        failures.into_boxed_slice(),
    );
    let actual = error
        .completion_diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(error.clone().completion_diagnostics(), error.completion_diagnostics());
    assert!(error.to_string().contains("first callback"));
    assert!(error.to_string().contains("non-string panic payload"));
    assert!(error.current().is_none());
}

#[test]
fn test_timeout_snapshot_is_optional_and_not_reloaded() {
    for snapshot in [None, Some(Arc::new(7usize))] {
        let expected = snapshot.as_deref().copied();
        let error = CasError::<usize, ()>::from_retry_parts(
            RetryErrorReason::TimedOut {
                scope: RetryTimeoutScope::Attempt,
            },
            Some(AttemptFailure::TimedOut {
                scope: RetryTimeoutScope::Attempt,
            }),
            RetryContext::new(1, 3),
            snapshot,
            Vec::new().into_boxed_slice(),
        );
        assert_eq!(error.current().map(|value| **value), expected);
        assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
    }
}

#[test]
fn test_error_source_still_refers_to_business_error() {
    use std::error::Error;
    let error = CasError::<usize, std::io::Error>::from_retry_parts(
        RetryErrorReason::Aborted,
        Some(AttemptFailure::Error(CasAttemptFailure::Abort {
            current: Arc::new(1),
            error: std::io::Error::other("business source"),
        })),
        RetryContext::new(1, 3),
        None,
        Vec::new().into_boxed_slice(),
    );
    assert_eq!(error.source().expect("business source").to_string(), "business source");
}
