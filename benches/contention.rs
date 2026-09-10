// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Real writer contention with successful throughput and inclusive tail
//! latency. Wall time includes barrier release, per-call clock sampling,
//! counters, result destruction, and joins. Setup and final assertions are
//! outside the measured interval; values are not absolute CI thresholds.

use std::hint::black_box;
use std::sync::Barrier;
use std::thread::scope;
use std::time::Instant;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutor;
use qubit_cas::CasStrategy;

const CALLS: usize = 10_000;
const WARMUP: usize = 1_000;
const ROUNDS: usize = 5;
const STRATEGY_SEED: usize = 0x5a17;
const CSV_COLUMNS: usize = 28;
const CSV_HEADER: &str = "record_type,os,arch,host_threads,round,writers,strategy,calls,successes,failures,conflicts,ops_per_sec,p50_ns,p95_ns,p99_ns,rounds,ops_per_sec_min,ops_per_sec_median,ops_per_sec_max,p50_min_ns,p50_median_ns,p50_max_ns,p95_min_ns,p95_median_ns,p95_max_ns,p99_min_ns,p99_median_ns,p99_max_ns";

const STRATEGIES: [CasStrategy; 3] = [
    CasStrategy::LatencyFirst,
    CasStrategy::ContentionBackoff,
    CasStrategy::ReliabilityFirst,
];

#[derive(Debug)]
struct RoundMetrics {
    throughput: f64,
    p50_ns: u128,
    p95_ns: u128,
    p99_ns: u128,
    successes: usize,
    failures: usize,
    conflicts: u64,
}

#[derive(Debug)]
struct BenchmarkEnvironment {
    os: &'static str,
    arch: &'static str,
    host_threads: usize,
}

/// Runs each preset with the same writer counts and fixed call budget.
fn main() {
    let environment = BenchmarkEnvironment {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        host_threads: std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1),
    };
    println!("{CSV_HEADER}");
    for writers in [1, 2, 4, 8] {
        let mut all_rounds: [Vec<RoundMetrics>; STRATEGIES.len()] = std::array::from_fn(|_| Vec::with_capacity(ROUNDS));
        for round in 0..ROUNDS {
            for offset in 0..STRATEGIES.len() {
                let strategy_index = (STRATEGY_SEED + round + offset) % STRATEGIES.len();
                let strategy = STRATEGIES[strategy_index];
                let metrics = run_round(writers, strategy);
                print_round(&environment, round, writers, strategy, &metrics);
                all_rounds[strategy_index].push(metrics);
            }
        }
        for (strategy, rounds) in STRATEGIES.into_iter().zip(all_rounds) {
            print_summary(&environment, writers, strategy, &rounds);
        }
    }
}

/// Writes one round record using the benchmark's fixed CSV schema.
///
/// `environment` supplies the host metadata, `round`, `writers`, and
/// `strategy` identify the workload, and `metrics` supplies its observations.
/// The function writes one `round` record to standard output.
fn print_round(
    environment: &BenchmarkEnvironment,
    round: usize,
    writers: usize,
    strategy: CasStrategy,
    metrics: &RoundMetrics,
) {
    write_csv_row([
        "round".to_owned(),
        environment.os.to_owned(),
        environment.arch.to_owned(),
        environment.host_threads.to_string(),
        round.to_string(),
        writers.to_string(),
        format!("{strategy:?}"),
        (writers * CALLS).to_string(),
        metrics.successes.to_string(),
        metrics.failures.to_string(),
        metrics.conflicts.to_string(),
        format!("{:.1}", metrics.throughput),
        metrics.p50_ns.to_string(),
        metrics.p95_ns.to_string(),
        metrics.p99_ns.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]);
}

/// Measures one concurrent workload; failure calls are not silently retried.
fn run_round(writers: usize, strategy: CasStrategy) -> RoundMetrics {
    let state = AtomicRef::from_value(black_box(0usize));
    let executor = CasExecutor::<usize, ()>::with_strategy(black_box(strategy));
    let ready = Barrier::new(writers + 1);
    let start_gate = Barrier::new(writers + 1);
    let (results, elapsed) = scope(|scope| {
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
                let mut unexpected_errors = 0usize;
                ready.wait();
                start_gate.wait();
                for _ in 0..CALLS {
                    let start = Instant::now();
                    let result = black_box(executor.execute_result(black_box(state), |current: &usize| {
                        CasDecision::update(black_box(*current) + 1, ())
                    }));
                    samples.push(start.elapsed().as_nanos());
                    match result {
                        Ok(success) => {
                            successes += 1;
                            conflicts += u64::from(success.attempts().saturating_sub(1));
                        }
                        Err(error) => {
                            unexpected_errors += usize::from(!matches!(
                                error.kind(),
                                CasErrorKind::ConflictExhausted
                                    | CasErrorKind::OperationBudgetExceeded
                                    | CasErrorKind::TotalBudgetExceeded
                            ));
                            conflicts += u64::from(error.attempts());
                        }
                    }
                }
                (samples, successes, conflicts, unexpected_errors)
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
    let mut unexpected_errors = 0;
    for (latencies, completed, failed_attempts, unexpected) in results {
        unexpected_errors += unexpected;
        samples.extend(latencies);
        successes += completed;
        conflicts += failed_attempts;
    }
    assert_eq!(unexpected_errors, 0, "unexpected failure category in CAS benchmark");
    samples.sort_unstable();
    assert_eq!(
        *state.load(),
        successes,
        "every successful update must publish exactly one increment"
    );
    let calls = writers * CALLS;
    let failures = calls - successes;
    let throughput = successes as f64 / elapsed.as_secs_f64();
    RoundMetrics {
        throughput,
        p50_ns: percentile(&samples, 50),
        p95_ns: percentile(&samples, 95),
        p99_ns: percentile(&samples, 99),
        successes,
        failures,
        conflicts,
    }
}

/// Prints the min/median/max throughput and tail-latency observations for a
/// writer/strategy combination.
///
/// # Parameters
///
/// - `environment` identifies the host configuration used for the samples.
/// - `writers` is the number of concurrent writer threads.
/// - `strategy` is the CAS retry strategy represented by the samples.
/// - `rounds` contains one independent measurement for each completed round.
///
/// # Panics
///
/// Panics when `rounds` is empty because an aggregate cannot be computed.
/// The function writes one `summary` record using the same fixed CSV schema as
/// round records.
fn print_summary(environment: &BenchmarkEnvironment, writers: usize, strategy: CasStrategy, rounds: &[RoundMetrics]) {
    assert!(!rounds.is_empty());
    let mut throughputs = rounds.iter().map(|round| round.throughput).collect::<Vec<_>>();
    let mut p50 = rounds.iter().map(|round| round.p50_ns).collect::<Vec<_>>();
    let mut p95 = rounds.iter().map(|round| round.p95_ns).collect::<Vec<_>>();
    let mut p99 = rounds.iter().map(|round| round.p99_ns).collect::<Vec<_>>();
    throughputs.sort_by(f64::total_cmp);
    p50.sort_unstable();
    p95.sort_unstable();
    p99.sort_unstable();
    write_csv_row([
        "summary".to_owned(),
        environment.os.to_owned(),
        environment.arch.to_owned(),
        environment.host_threads.to_string(),
        String::new(),
        writers.to_string(),
        format!("{strategy:?}"),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        rounds.len().to_string(),
        format!("{:.1}", throughputs[0]),
        format!("{:.1}", median(&throughputs)),
        format!("{:.1}", throughputs[throughputs.len() - 1]),
        p50[0].to_string(),
        median(&p50).to_string(),
        p50[p50.len() - 1].to_string(),
        p95[0].to_string(),
        median(&p95).to_string(),
        p95[p95.len() - 1].to_string(),
        p99[0].to_string(),
        median(&p99).to_string(),
        p99[p99.len() - 1].to_string(),
    ]);
}

/// Writes a single record with exactly the columns declared by `CSV_HEADER`.
///
/// `fields` is a fixed-size array, so both round and summary records always
/// contain the same number of columns. The function writes the record to
/// standard output.
fn write_csv_row(fields: [String; CSV_COLUMNS]) {
    println!("{}", fields.join(","));
}

/// Returns the upper middle element of a non-empty sorted sample set.
///
/// # Parameters
///
/// - `sorted` must contain samples in ascending order and must not be empty.
///
/// # Panics
///
/// Panics when `sorted` is empty. For an even number of samples, the upper of
/// the two middle elements is selected.
fn median<T: Copy>(sorted: &[T]) -> T {
    sorted[sorted.len() / 2]
}

/// Returns an upper-index observation from sorted latencies, including
/// failures.
fn percentile(sorted: &[u128], percent: usize) -> u128 {
    assert!(!sorted.is_empty());
    let index = ((sorted.len() - 1) * percent).div_ceil(100);
    sorted[index]
}
