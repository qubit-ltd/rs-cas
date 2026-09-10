// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures the overhead of result-only execution and optional CAS
//! observability for uncontended and deliberately conflicted updates.

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

    let raw = measure_raw(force_conflict);
    let result_only = measure_result_executor(benchmark_executor(), force_conflict);
    let report_only = measure_executor(benchmark_executor(), CasHooks::new(), force_conflict);
    let event_empty = measure_executor(
        benchmark_executor(),
        CasHooks::new().on_event(|_: &CasEvent| {}),
        force_conflict,
    );
    let event_light = measure_executor(benchmark_executor(), light_event_hook(), force_conflict);
    let event_and_alert_light = measure_executor(benchmark_executor(), light_alert_hooks(), force_conflict);

    print_row("raw_cas_floor", &raw, None, None);
    print_row("result_only", &result_only, Some(raw.ops_per_sec), None);
    print_row("report_only", &report_only, Some(raw.ops_per_sec), None);
    print_row(
        "event_noop_listener",
        &event_empty,
        Some(raw.ops_per_sec),
        Some(report_only.ops_per_sec),
    );
    print_row(
        "event_stream_light",
        &event_light,
        Some(raw.ops_per_sec),
        Some(report_only.ops_per_sec),
    );
    print_row(
        "event_and_alert_light",
        &event_and_alert_light,
        Some(raw.ops_per_sec),
        Some(report_only.ops_per_sec),
    );
}

/// Builds the bounded no-delay executor shared by measured workloads.
fn benchmark_executor() -> CasExecutor<usize, &'static str> {
    CasExecutor::<usize, &'static str>::builder()
        .max_attempts(100)
        .no_delay()
        .build()
        .expect("benchmark retry policy should be valid")
}

/// Warms the result-only path and returns the median sample throughput.
fn measure_result_executor(executor: CasExecutor<usize, &'static str>, force_conflict: bool) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        run_result_executor_sample(executor.clone(), force_conflict).expect("benchmark warmup should succeed");
    }

    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    let mut last = None;
    for _ in 0..MEASURED_RUNS {
        let result =
            run_result_executor_sample(executor.clone(), force_conflict).expect("benchmark sample should succeed");
        samples.push(result.ops_per_sec);
        last = Some(result);
    }

    let ops_per_sec = median(&mut samples);
    let mut result = last.expect("at least one benchmark sample should run");
    result.ops_per_sec = ops_per_sec;
    result.ns_per_op = 1_000_000_000.0 / ops_per_sec;
    result
}

/// Measures result-only calls, including retry bookkeeping and result
/// destruction.
fn run_result_executor_sample(
    executor: CasExecutor<usize, &'static str>,
    force_conflict: bool,
) -> Result<BenchResult, String> {
    let state = AtomicRef::from_value(0usize);
    let forced = AtomicUsize::new(0);
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let success = executor
            .execute_result(&state, |current: &usize| {
                if force_conflict && forced.fetch_add(1, Ordering::Relaxed).is_multiple_of(2) {
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

/// Warms the observed path and reports median throughput across samples.
fn measure_executor(executor: CasExecutor<usize, &'static str>, hooks: CasHooks, force_conflict: bool) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        run_executor_sample(executor.clone(), hooks.clone(), force_conflict).expect("benchmark warmup should succeed");
    }

    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    let mut last = None;
    for _ in 0..MEASURED_RUNS {
        let result = run_executor_sample(executor.clone(), hooks.clone(), force_conflict)
            .expect("benchmark sample should succeed");
        samples.push(result.ops_per_sec);
        last = Some(result);
    }

    let ops_per_sec = median(&mut samples);
    let mut result = last.expect("at least one benchmark sample should run");
    result.ops_per_sec = ops_per_sec;
    result.ns_per_op = 1_000_000_000.0 / ops_per_sec;
    result
}

/// Measures observed calls, including report and hook costs.
fn run_executor_sample(
    executor: CasExecutor<usize, &'static str>,
    hooks: CasHooks,
    force_conflict: bool,
) -> Result<BenchResult, String> {
    let state = AtomicRef::from_value(0usize);
    let forced = AtomicUsize::new(0);
    // Instant is a monotonic clock, so elapsed measurements are not affected by
    // wall-clock jumps.
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let outcome = executor.execute_with_hooks(
            &state,
            |current: &usize| {
                if force_conflict && forced.fetch_add(1, Ordering::Relaxed).is_multiple_of(2) {
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
    let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();
    Ok(BenchResult {
        ops_per_sec,
        ns_per_op: elapsed.as_nanos() as f64 / ITERATIONS as f64,
        avg_attempts: attempts as f64 / ITERATIONS as f64,
        conflicts,
    })
}

/// Warms the raw CAS floor and reports median throughput across samples.
fn measure_raw(force_conflict: bool) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        let _ = run_raw_sample(force_conflict);
    }

    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    let mut last = None;
    for _ in 0..MEASURED_RUNS {
        let result = run_raw_sample(force_conflict);
        samples.push(result.ops_per_sec);
        last = Some(result);
    }

    let ops_per_sec = median(&mut samples);
    let mut result = last.expect("at least one benchmark sample should run");
    result.ops_per_sec = ops_per_sec;
    result.ns_per_op = 1_000_000_000.0 / ops_per_sec;
    result
}

/// Measures raw CAS; the controlled writer forces at most one conflict per
/// update.
fn run_raw_sample(force_conflict: bool) -> BenchResult {
    let state = AtomicRef::from_value(0usize);
    let forced = AtomicUsize::new(0);
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let mut first_attempt = true;
        loop {
            attempts += 1;
            let current = state.load();
            if force_conflict && first_attempt {
                forced.fetch_add(1, Ordering::Relaxed);
                state.store(Arc::new(*current + 1));
            }
            first_attempt = false;
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
    let ops_per_sec = ITERATIONS as f64 / elapsed.as_secs_f64();
    BenchResult {
        ops_per_sec,
        ns_per_op: elapsed.as_nanos() as f64 / ITERATIONS as f64,
        avg_attempts: attempts as f64 / ITERATIONS as f64,
        conflicts,
    }
}

/// Sorts finite, nonempty samples and returns the middle observation.
fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(|left, right| left.partial_cmp(right).expect("benchmark samples should not be NaN"));
    samples[samples.len() / 2]
}

/// Prints one measured path and its optional baseline comparisons.
fn print_row(name: &'static str, result: &BenchResult, raw_ops: Option<f64>, report_ops: Option<f64>) {
    let raw_loss = raw_ops.map(|baseline| loss_percent(result.ops_per_sec, baseline));
    let report_loss = report_ops.map(|baseline| loss_percent(result.ops_per_sec, baseline));
    println!(
        "{name:24} ops/s={:>10.0} ns/op={:>8.1} avg_attempts={:.3} conflicts={:<8} loss_vs_raw={} loss_vs_report_only={}",
        result.ops_per_sec,
        result.ns_per_op,
        result.avg_attempts,
        result.conflicts,
        format_loss(raw_loss),
        format_loss(report_loss),
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
