// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures the overhead of result-only execution and optional CAS
//! observability for uncontended and deliberately conflicted updates.

use std::array;
use std::cell::Cell;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Instant;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAlert;
use qubit_cas::CasDecision;
use qubit_cas::CasEvent;
use qubit_cas::CasExecutor;
use qubit_cas::CasHooks;
use qubit_cas::ContentionThresholds;

const ITERATIONS: usize = 200_000;
const WARMUP_RUNS: usize = 2;
const MEASURED_RUNS: usize = 9;

/// Runs both deterministic contention scenarios and prints median throughput.
fn main() {
    println!("iterations_per_sample={ITERATIONS}, warmups={WARMUP_RUNS}, samples={MEASURED_RUNS}");

    run_group("low_conflict", false);
    run_group("forced_conflict", true);
}

/// Compares raw, result, report, and listener costs using identical inputs.
fn run_group(group: &'static str, force_conflict: bool) {
    println!();
    println!("## {group}");

    let executors: [_; 5] = array::from_fn(|_| benchmark_executor());
    let hooks = [
        CasHooks::new(),
        CasHooks::new().on_event(|_: &CasEvent| {}),
        light_event_hook(),
        light_alert_hooks(),
    ];
    let mut samples: [Vec<BenchResult>; 6] = array::from_fn(|_| Vec::with_capacity(MEASURED_RUNS));
    for measuring in [false, true] {
        let rounds = if measuring { MEASURED_RUNS } else { WARMUP_RUNS };
        for round in 0..rounds {
            for offset in 0..6 {
                let variant = (round + offset) % 6;
                let sample = match variant {
                    0 => run_raw_sample(force_conflict),
                    1 => run_result_executor_sample(executors[0].clone(), force_conflict)
                        .expect("benchmark sample should succeed"),
                    _ => run_executor_sample(
                        executors[variant - 1].clone(),
                        hooks[variant - 2].clone(),
                        force_conflict,
                    )
                    .expect("benchmark sample should succeed"),
                };
                if measuring {
                    samples[variant].push(sample);
                }
            }
        }
    }
    for values in &mut samples {
        assert_eq!(values.len(), MEASURED_RUNS);
        assert!(
            values
                .iter()
                .all(|value| value.ops_per_sec.is_finite() && value.ops_per_sec > 0.0)
        );
        values.sort_by(|left, right| left.ops_per_sec.total_cmp(&right.ops_per_sec));
    }
    let raw_ops = samples[0][MEASURED_RUNS / 2].ops_per_sec;
    let report_ops = samples[2][MEASURED_RUNS / 2].ops_per_sec;
    let names = [
        "raw_cas_floor",
        "result_only",
        "report_only",
        "event_noop_listener",
        "event_stream_light",
        "event_and_alert_light",
    ];
    for (index, name) in names.into_iter().enumerate() {
        print_row(
            name,
            &samples[index],
            (index > 0).then_some(raw_ops),
            (index >= 3).then_some(report_ops),
        );
    }
}

/// Builds the bounded no-delay executor shared by measured workloads.
fn benchmark_executor() -> CasExecutor<usize, &'static str> {
    CasExecutor::<usize, &'static str>::builder()
        .max_attempts(100)
        .no_delay()
        .build()
        .expect("benchmark retry policy should be valid")
}

/// Measures result-only calls, including retry bookkeeping and result
/// destruction.
fn run_result_executor_sample(
    executor: CasExecutor<usize, &'static str>,
    force_conflict: bool,
) -> Result<BenchResult, String> {
    let state = AtomicRef::from_value(0usize);
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let first_attempt = Cell::new(true);
        let success = executor
            .execute_result(&state, |current: &usize| {
                let was_first = first_attempt.replace(false);
                if force_conflict && was_first {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, *current + 1)
            })
            .map_err(|error| format!("result-only CAS execution failed: {error:?}"))?;
        attempts += u64::from(success.attempts());
        conflicts += u64::from(success.attempts().saturating_sub(1));
        black_box(success);
    }

    let elapsed = start.elapsed();
    let expected_attempts = if force_conflict { 2 * ITERATIONS } else { ITERATIONS };
    let expected_conflicts = if force_conflict { ITERATIONS } else { 0 };
    assert_eq!(attempts, expected_attempts as u64);
    assert_eq!(conflicts, expected_conflicts as u64);
    assert_eq!(*state.load(), expected_attempts);
    let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();
    Ok(BenchResult {
        ops_per_sec,
        ns_per_op: elapsed.as_nanos() as f64 / ITERATIONS as f64,
        avg_attempts: attempts as f64 / ITERATIONS as f64,
        conflicts,
    })
}

/// Registers both light event accounting and contention alert accounting.
fn light_alert_hooks() -> CasHooks {
    let alerts = Arc::new(AtomicUsize::new(0));
    let alert_count = Arc::clone(&alerts);
    light_event_hook().on_contention_alert(ContentionThresholds::new(2, 1, 0.5), move |alert: &CasAlert| {
        alert_count.fetch_add(alert.report().conflicts() as usize, Ordering::Relaxed);
    })
}

/// Counts failed-attempt events with a relaxed atomic increment.
fn light_event_hook() -> CasHooks {
    let events = Arc::new(AtomicUsize::new(0));
    let event_count = Arc::clone(&events);
    CasHooks::new().on_event(move |event: &CasEvent| {
        if matches!(event, CasEvent::AttemptFailed { .. }) {
            event_count.fetch_add(1, Ordering::Relaxed);
        }
    })
}

#[derive(Debug, Clone, Copy)]
struct BenchResult {
    ops_per_sec: f64,
    ns_per_op: f64,
    avg_attempts: f64,
    conflicts: u64,
}

/// Measures observed calls, including report and hook costs.
fn run_executor_sample(
    executor: CasExecutor<usize, &'static str>,
    hooks: CasHooks,
    force_conflict: bool,
) -> Result<BenchResult, String> {
    let state = AtomicRef::from_value(0usize);
    // Instant is a monotonic clock, so elapsed measurements are not affected by
    // wall-clock jumps.
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let first_attempt = Cell::new(true);
        let outcome = executor.execute_with_hooks(
            &state,
            |current: &usize| {
                let was_first = first_attempt.replace(false);
                if force_conflict && was_first {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, *current + 1)
            },
            hooks.clone(),
        );
        attempts += u64::from(outcome.report().attempts_total());
        conflicts += u64::from(outcome.report().conflicts());
        black_box(
            outcome
                .into_result()
                .map_err(|error| format!("reported CAS execution failed: {error:?}"))?,
        );
    }

    let elapsed = start.elapsed();
    let expected_attempts = if force_conflict { 2 * ITERATIONS } else { ITERATIONS };
    let expected_conflicts = if force_conflict { ITERATIONS } else { 0 };
    assert_eq!(attempts, expected_attempts as u64);
    assert_eq!(conflicts, expected_conflicts as u64);
    assert_eq!(*state.load(), expected_attempts);
    let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();
    Ok(BenchResult {
        ops_per_sec,
        ns_per_op: elapsed.as_nanos() as f64 / ITERATIONS as f64,
        avg_attempts: attempts as f64 / ITERATIONS as f64,
        conflicts,
    })
}

/// Measures raw CAS; the controlled writer forces at most one conflict per
/// update.
fn run_raw_sample(force_conflict: bool) -> BenchResult {
    let state = AtomicRef::from_value(0usize);
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let first_attempt = Cell::new(true);
        loop {
            attempts += 1;
            let current = state.load();
            let was_first = first_attempt.replace(false);
            if force_conflict && was_first {
                state.store(Arc::new(*current + 1));
            }
            let next = Arc::new(*current + 1);
            match state.compare_set(&current, Arc::clone(&next)) {
                Ok(()) => {
                    black_box(next);
                    break;
                }
                Err(_) => conflicts += 1,
            }
        }
    }

    let elapsed = start.elapsed();
    let expected_attempts = if force_conflict { 2 * ITERATIONS } else { ITERATIONS };
    let expected_conflicts = if force_conflict { ITERATIONS } else { 0 };
    assert_eq!(attempts, expected_attempts as u64);
    assert_eq!(conflicts, expected_conflicts as u64);
    assert_eq!(*state.load(), expected_attempts);
    let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();
    BenchResult {
        ops_per_sec,
        ns_per_op: elapsed.as_nanos() as f64 / ITERATIONS as f64,
        avg_attempts: attempts as f64 / ITERATIONS as f64,
        conflicts,
    }
}

/// Prints a sorted, nonempty sample set's median, range, and optional baseline
/// comparisons.
fn print_row(name: &'static str, samples: &[BenchResult], raw_ops: Option<f64>, report_ops: Option<f64>) {
    let result = &samples[samples.len() / 2];
    let minimum = samples.first().expect("nonempty samples").ops_per_sec;
    let maximum = samples.last().expect("nonempty samples").ops_per_sec;
    assert!(minimum <= result.ops_per_sec && result.ops_per_sec <= maximum);
    let raw_loss = raw_ops.map(|baseline| loss_percent(result.ops_per_sec, baseline));
    let report_loss = report_ops.map(|baseline| loss_percent(result.ops_per_sec, baseline));
    println!(
        "{name:24} ops/s={:>10.0} ns/op={:>8.1} avg_attempts={:.3} conflicts={:<8} loss_vs_raw={} loss_vs_report_only={} ops/s_min={:.0} ops/s_max={:.0}",
        result.ops_per_sec,
        result.ns_per_op,
        result.avg_attempts,
        result.conflicts,
        format_loss(raw_loss),
        format_loss(report_loss),
        minimum,
        maximum,
    );
}

/// Computes throughput loss relative to a positive baseline.
fn loss_percent(ops_per_sec: f64, baseline_ops_per_sec: f64) -> f64 {
    (baseline_ops_per_sec - ops_per_sec) / baseline_ops_per_sec * 100.0
}

/// Formats a measured loss or indicates that no baseline is available.
fn format_loss(loss: Option<f64>) -> String {
    match loss {
        Some(value) => format!("{value:>6.2}%"),
        None => "   n/a".to_string(),
    }
}
