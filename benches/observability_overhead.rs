// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use std::time::Instant;

use qubit_atomic::AtomicRef;
use qubit_cas::{
    CasDecision,
    CasEvent,
    CasExecutor,
    CasHooks,
    CasObservabilityConfig,
    ContentionThresholds,
};

const ITERATIONS: usize = 200_000;
const WARMUP_RUNS: usize = 2;
const MEASURED_RUNS: usize = 9;

fn main() {
    println!(
        "iterations_per_sample={ITERATIONS}, warmups={WARMUP_RUNS}, samples={MEASURED_RUNS}"
    );

    run_group("low_conflict", false);
    run_group("forced_conflict", true);
}

fn run_group(group: &'static str, force_conflict: bool) {
    println!();
    println!("## {group}");

    let raw = measure_raw(force_conflict);
    let result_only = measure_result_executor(
        CasExecutor::<usize, &'static str>::latency_first(),
        force_conflict,
    );
    let report_only = measure_executor(
        CasExecutor::<usize, &'static str>::latency_first(),
        CasHooks::new(),
        force_conflict,
    );
    let event_empty = measure_executor(
        CasExecutor::<usize, &'static str>::builder()
            .observability(CasObservabilityConfig::event_stream())
            .build_latency_first()
            .expect("benchmark executor should build"),
        CasHooks::new(),
        force_conflict,
    );
    let event_light = measure_executor(
        CasExecutor::<usize, &'static str>::builder()
            .observability(CasObservabilityConfig::event_stream())
            .build_latency_first()
            .expect("benchmark executor should build"),
        light_event_hook(),
        force_conflict,
    );
    let alert_light = measure_executor(
        CasExecutor::<usize, &'static str>::builder()
            .observability(CasObservabilityConfig::event_stream_with_alert(
                ContentionThresholds::new(2, 1, 0.5),
            ))
            .build_latency_first()
            .expect("benchmark executor should build"),
        light_event_hook(),
        force_conflict,
    );

    print_row("raw_cas_floor", &raw, None, None);
    print_row("result_only", &result_only, Some(raw.ops_per_sec), None);
    print_row("report_only", &report_only, Some(raw.ops_per_sec), None);
    print_row(
        "event_stream_empty",
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
        "alert_light",
        &alert_light,
        Some(raw.ops_per_sec),
        Some(report_only.ops_per_sec),
    );
}

fn measure_result_executor(
    executor: CasExecutor<usize, &'static str>,
    force_conflict: bool,
) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        let _ = run_result_executor_sample(executor.clone(), force_conflict);
    }

    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    let mut last = None;
    for _ in 0..MEASURED_RUNS {
        let result =
            run_result_executor_sample(executor.clone(), force_conflict);
        samples.push(result.ops_per_sec);
        last = Some(result);
    }

    let ops_per_sec = median(&mut samples);
    let mut result = last.expect("at least one benchmark sample should run");
    result.ops_per_sec = ops_per_sec;
    result.ns_per_op = 1_000_000_000.0 / ops_per_sec;
    result
}

fn run_result_executor_sample(
    executor: CasExecutor<usize, &'static str>,
    force_conflict: bool,
) -> BenchResult {
    let state = AtomicRef::from_value(0usize);
    let forced = AtomicUsize::new(0);
    let start = Instant::now();
    let mut attempts = 0u64;
    let mut conflicts = 0u64;

    for _ in 0..ITERATIONS {
        let success = executor
            .execute_result(&state, |current: &usize| {
                if force_conflict
                    && forced.fetch_add(1, Ordering::Relaxed).is_multiple_of(2)
                {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, *current + 1)
            })
            .expect("benchmark CAS execution should succeed");
        attempts += u64::from(success.attempts());
        conflicts += u64::from(success.attempts().saturating_sub(1));
        black_box(success);
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

fn measure_executor(
    executor: CasExecutor<usize, &'static str>,
    hooks: CasHooks,
    force_conflict: bool,
) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        let _ = run_executor_sample(
            executor.clone(),
            hooks.clone(),
            force_conflict,
        );
    }

    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    let mut last = None;
    for _ in 0..MEASURED_RUNS {
        let result = run_executor_sample(
            executor.clone(),
            hooks.clone(),
            force_conflict,
        );
        samples.push(result.ops_per_sec);
        last = Some(result);
    }

    let ops_per_sec = median(&mut samples);
    let mut result = last.expect("at least one benchmark sample should run");
    result.ops_per_sec = ops_per_sec;
    result.ns_per_op = 1_000_000_000.0 / ops_per_sec;
    result
}

fn run_executor_sample(
    executor: CasExecutor<usize, &'static str>,
    hooks: CasHooks,
    force_conflict: bool,
) -> BenchResult {
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
                if force_conflict
                    && forced.fetch_add(1, Ordering::Relaxed).is_multiple_of(2)
                {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, *current + 1)
            },
            hooks.clone(),
        );
        attempts += u64::from(outcome.report().attempts_total());
        conflicts += u64::from(outcome.report().conflicts());
        black_box(outcome.expect("benchmark CAS execution should succeed"));
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

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(|left, right| {
        left.partial_cmp(right)
            .expect("benchmark samples should not be NaN")
    });
    samples[samples.len() / 2]
}

fn print_row(
    name: &'static str,
    result: &BenchResult,
    raw_ops: Option<f64>,
    report_ops: Option<f64>,
) {
    let raw_loss =
        raw_ops.map(|baseline| loss_percent(result.ops_per_sec, baseline));
    let report_loss =
        report_ops.map(|baseline| loss_percent(result.ops_per_sec, baseline));
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

fn loss_percent(ops_per_sec: f64, baseline_ops_per_sec: f64) -> f64 {
    (baseline_ops_per_sec - ops_per_sec) / baseline_ops_per_sec * 100.0
}

fn format_loss(loss: Option<f64>) -> String {
    match loss {
        Some(value) => format!("{value:>6.2}%"),
        None => "   n/a".to_string(),
    }
}
