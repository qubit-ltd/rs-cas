// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Concurrent snapshot publication and replay contracts.

use std::sync::Arc;
use std::sync::Barrier;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread::scope;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

struct DropTrackedOutput {
    attempt_snapshot: usize,
    drops: Arc<AtomicUsize>,
}

impl Drop for DropTrackedOutput {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_concurrent_updates_return_committed_pairs() {
    let state = AtomicRef::from_value(0usize);
    let gate = Barrier::new(4);
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(10_000)
        .no_delay()
        .build()
        .expect("valid policy");
    let mut previous = scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..4 {
            let state = &state;
            let gate = &gate;
            let executor = &executor;
            handles.push(scope.spawn(move || {
                let first = AtomicBool::new(true);
                let mut committed = Vec::new();
                for _ in 0..100 {
                    let ok = executor
                        .execute_result(state, |current: &usize| {
                            if first.swap(false, Ordering::SeqCst) {
                                gate.wait();
                            }
                            CasDecision::update(*current + 1, *current + 1)
                        })
                        .expect("bounded contention must succeed");
                    let old = **ok.previous().expect("update has previous state");
                    assert_eq!(**ok.current(), old + 1);
                    assert_eq!(*ok.output(), old + 1);
                    committed.push(old);
                }
                committed
            }));
        }
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("writer must finish"))
            .collect::<Vec<_>>()
    });
    previous.sort_unstable();
    assert_eq!(previous, (0..400).collect::<Vec<_>>());
    assert_eq!(*state.load(), 400);
}

#[test]
fn test_conflict_replays_operation_from_latest_snapshot() {
    let state = AtomicRef::from_value(0usize);
    let first = AtomicBool::new(true);
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .build()
        .expect("valid policy");
    let ok = executor
        .execute_result(&state, |current: &usize| {
            if first.swap(false, Ordering::SeqCst) {
                state.store(Arc::new(7));
            }
            CasDecision::update(*current + 1, *current)
        })
        .expect("second attempt must succeed");
    assert_eq!(ok.attempts(), 2);
    assert_eq!(**ok.previous().expect("update has previous snapshot"), 7);
    assert_eq!(**ok.current(), 8);
    assert_eq!(*ok.output(), 7);
}

#[test]
fn test_conflicted_attempt_drops_non_clone_output_and_returns_only_winner() {
    let state = AtomicRef::from_value(0usize);
    let first = AtomicBool::new(true);
    let drops = Arc::new(AtomicUsize::new(0));
    let executor = CasExecutor::<usize, ()>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("valid policy");

    let success = executor
        .execute_result(&state, |current: &usize| {
            if first.swap(false, Ordering::SeqCst) {
                state.store(Arc::new(7));
            }
            CasDecision::update(
                *current + 1,
                DropTrackedOutput {
                    attempt_snapshot: *current,
                    drops: Arc::clone(&drops),
                },
            )
        })
        .expect("the replayed attempt should commit");

    assert_eq!(2, success.attempts());
    assert_eq!(
        1,
        drops.load(Ordering::SeqCst),
        "failed attempt output must be released"
    );

    let output = success.into_output();
    assert_eq!(7, output.attempt_snapshot, "caller receives only the winning output");
    assert_eq!(1, drops.load(Ordering::SeqCst), "winning output remains caller-owned");
    drop(output);
    assert_eq!(2, drops.load(Ordering::SeqCst), "caller releases the winning output");
}

#[test]
fn test_finish_returns_observed_snapshot_without_revalidation() {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, ()>::latency_first();
    let ok = executor
        .execute_result(&state, |current: &usize| {
            state.store(Arc::new(9));
            CasDecision::finish(*current)
        })
        .expect("finish succeeds");
    assert!(!ok.is_updated());
    assert_eq!(**ok.current(), 0);
    assert_eq!(*ok.output(), 0);
    assert_eq!(*state.load(), 9);
}

#[test]
fn test_equal_value_update_still_publishes_new_snapshot() {
    let state = AtomicRef::from_value(3usize);
    let old = state.load();
    let ok = CasExecutor::<usize, ()>::latency_first()
        .execute_result(&state, |current: &usize| CasDecision::update(*current, ()))
        .expect("same-value publication succeeds");
    assert!(ok.is_updated());
    assert!(!Arc::ptr_eq(&old, ok.current()));
    assert!(Arc::ptr_eq(&state.load(), ok.current()));
}
