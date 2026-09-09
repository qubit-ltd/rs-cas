// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Real writer contention with successful throughput and inclusive tail
//! latency.

use std::sync::Barrier;
use std::time::Instant;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;
use qubit_cas::CasStrategy;

const CALLS: usize = 10_000;
const WARMUP: usize = 1_000;

/// Runs each preset with the same writer counts and fixed call budget.
fn main() {
    println!("threads,strategy,calls,successes,failures,conflicts,ops_per_sec,p50_ns,p95_ns,p99_ns");
    for writers in [1, 2, 4, 8] {
        for strategy in [
            CasStrategy::LatencyFirst,
            CasStrategy::ContentionBackoff,
            CasStrategy::ReliabilityFirst,
        ] {
            run(writers, strategy);
        }
    }
}

/// Measures one concurrent workload; failure calls are not silently retried.
fn run(writers: usize, strategy: CasStrategy) {
    let state = AtomicRef::from_value(0usize);
    let executor = CasExecutor::<usize, ()>::with_strategy(strategy);
    let ready = Barrier::new(writers + 1);
    let start_gate = Barrier::new(writers + 1);
    let (results, elapsed) = std::thread::scope(|scope| {
        let mut threads = Vec::with_capacity(writers);
        for _ in 0..writers {
            let state = &state;
            let executor = &executor;
            let ready = &ready;
            let start_gate = &start_gate;
            threads.push(scope.spawn(move || {
                let warmup_state = AtomicRef::from_value(0usize);
                for _ in 0..WARMUP {
                    executor
                        .execute_result(&warmup_state, |current: &usize| CasDecision::update(*current + 1, ()))
                        .expect("uncontended warmup");
                }
                let mut samples = Vec::with_capacity(CALLS);
                let mut successes = 0usize;
                let mut conflicts = 0u64;
                ready.wait();
                start_gate.wait();
                for _ in 0..CALLS {
                    let start = Instant::now();
                    let result =
                        executor.execute_result(state, |current: &usize| CasDecision::update(*current + 1, ()));
                    samples.push(start.elapsed().as_nanos());
                    match result {
                        Ok(success) => {
                            successes += 1;
                            conflicts += u64::from(success.attempts().saturating_sub(1));
                        }
                        Err(error) => {
                            assert!(matches!(
                                error.kind(),
                                CasErrorKind::ConflictExhausted
                                    | CasErrorKind::OperationBudgetExceeded
                                    | CasErrorKind::TotalBudgetExceeded
                            ));
                            conflicts += u64::from(error.attempts());
                        }
                    }
                }
                (samples, successes, conflicts)
            }));
        }
        ready.wait();
        let start = Instant::now();
        start_gate.wait();
        let results = threads
            .into_iter()
            .map(|thread| thread.join().expect("writer succeeded"))
            .collect::<Vec<_>>();
        (results, start.elapsed())
    });
    let mut samples = Vec::with_capacity(writers * CALLS);
    let mut successes = 0;
    let mut conflicts = 0;
    for (latencies, completed, failed_attempts) in results {
        samples.extend(latencies);
        successes += completed;
        conflicts += failed_attempts;
    }
    samples.sort_unstable();
    assert_eq!(
        *state.load(),
        successes,
        "every successful update must publish exactly one increment"
    );
    let calls = writers * CALLS;
    let failures = calls - successes;
    let throughput = successes as f64 / elapsed.as_secs_f64();
    println!(
        "{writers},{strategy:?},{calls},{successes},{failures},{conflicts},{throughput:.1},{},{},{}",
        percentile(&samples, 50),
        percentile(&samples, 95),
        percentile(&samples, 99)
    );
}

/// Returns a nearest-rank observation from sorted latencies, including
/// failures.
fn percentile(sorted: &[u128], percent: usize) -> u128 {
    assert!(!sorted.is_empty());
    let index = ((sorted.len() - 1) * percent).div_ceil(100);
    sorted[index]
}
