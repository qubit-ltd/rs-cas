// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasRetryFailure;
use qubit_retry::RetryCallbackFailure;
use qubit_retry::RetryCallbackKind;
use qubit_retry::RetryCallbackPhase;
use qubit_retry::RetryCancellationPhase;
use qubit_retry::RetryInfrastructureFailure;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryPanic;
use qubit_retry::RetryTimeoutScope;

/// Verifies structured retry terminal accessors preserve their exact details.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_retry_failure_accessors_preserve_terminal_details() {
    let exhausted = CasRetryFailure::Exhausted {
        limit: RetryLimitKind::Attempts,
    };
    let timed_out = CasRetryFailure::TimedOut {
        scope: RetryTimeoutScope::Flow,
    };
    let cancelled = CasRetryFailure::Cancelled {
        phase: RetryCancellationPhase::Backoff,
    };
    let callback = RetryCallbackFailure::new(
        RetryCallbackKind::Observer,
        2,
        RetryCallbackPhase::AttemptFailed,
        RetryPanic::StaticStr("listener failed"),
    );
    let callback_failed = CasRetryFailure::CallbackFailed {
        callback: callback.clone(),
    };
    let infrastructure = RetryInfrastructureFailure::Clock {
        message: "offline".into(),
    };
    let infrastructure_failed = CasRetryFailure::Infrastructure {
        failure: infrastructure.clone(),
    };

    assert_eq!(exhausted.limit(), Some(RetryLimitKind::Attempts));
    assert_eq!(timed_out.timeout_scope(), Some(RetryTimeoutScope::Flow));
    assert_eq!(cancelled.cancellation_phase(), Some(RetryCancellationPhase::Backoff));
    assert_eq!(callback_failed.callback_failure(), Some(&callback));
    assert_eq!(infrastructure_failed.infrastructure_failure(), Some(&infrastructure));
    assert_eq!(CasRetryFailure::Aborted.limit(), None);
    assert_eq!(CasRetryFailure::Aborted.timeout_scope(), None);
    assert_eq!(CasRetryFailure::Aborted.cancellation_phase(), None);
    assert_eq!(CasRetryFailure::Aborted.callback_failure(), None);
    assert_eq!(CasRetryFailure::Aborted.infrastructure_failure(), None);
    assert_eq!(CasRetryFailure::Unknown.limit(), None);
    assert_eq!(CasRetryFailure::Unknown.timeout_scope(), None);
    assert_eq!(CasRetryFailure::Unknown.cancellation_phase(), None);
    assert_eq!(CasRetryFailure::Unknown.callback_failure(), None);
    assert_eq!(CasRetryFailure::Unknown.infrastructure_failure(), None);
}

/// Verifies every structured retry terminal renders diagnostic detail.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_retry_failure_display_describes_each_terminal() {
    let callback = RetryCallbackFailure::new(
        RetryCallbackKind::Rule,
        1,
        RetryCallbackPhase::RuleDecision,
        RetryPanic::StaticStr("rule failed"),
    );
    let failures = [
        CasRetryFailure::Aborted,
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::OperationElapsed,
        },
        CasRetryFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt,
        },
        CasRetryFailure::Cancelled {
            phase: RetryCancellationPhase::BeforeAttempt,
        },
        CasRetryFailure::CallbackFailed { callback },
        CasRetryFailure::Infrastructure {
            failure: RetryInfrastructureFailure::Timer {
                message: "timer offline".into(),
            },
        },
        CasRetryFailure::Unknown,
    ];

    let displays: Vec<String> = failures.iter().map(ToString::to_string).collect();
    assert_eq!(displays[0], "retry aborted");
    assert!(displays[1].contains("operation elapsed"));
    assert!(displays[2].contains("attempt"));
    assert!(displays[3].contains("before attempt"));
    assert!(displays[4].contains("rule callback 1"));
    assert!(displays[5].contains("timer offline"));
    assert_eq!(displays[6], "unknown retry terminal failure");
}
