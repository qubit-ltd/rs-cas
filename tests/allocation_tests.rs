// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Isolated, thread-local allocation measurements of public CAS execution.

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::cell::Cell;
use std::sync::Arc;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasExecutor;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;

/// Records allocations only inside the current thread's explicit window.
fn record() {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
        }
    });
}

// SAFETY: Every allocation and pointer is forwarded unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: The caller supplies the same valid layout passed to System.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: This allocator only returns pointers allocated by System.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        // SAFETY: The requested layout is forwarded without alteration.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        // SAFETY: System owns the pointer and receives the caller's original layout.
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Disables counting on success or unwinding.
struct Disable;
impl Drop for Disable {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(false));
    }
}

/// Returns the result and allocation count, excluding result destruction.
fn allocations<F, R>(operation: F) -> (R, usize)
where
    F: FnOnce() -> R,
{
    ALLOCS.with(|n| n.set(0));
    ACTIVE.with(|active| active.set(true));
    let disable = Disable;
    let result = operation();
    drop(disable);
    (result, ALLOCS.with(Cell::get))
}

#[test]
fn test_sync_allocation_measurements() {
    let (executor, constructor) = allocations(|| CasExecutor::<usize, ()>::builder().build().expect("valid policy"));
    let state = AtomicRef::from_value(0usize);
    let (first, cold) = allocations(|| executor.execute_result(&state, |_: &usize| CasDecision::finish(())));
    first.expect("first finish");
    let (result, warm) = allocations(|| executor.execute_result(&state, |_: &usize| CasDecision::finish(())));
    result.expect("warm finish");
    let (result, rich) = allocations(|| executor.execute(&state, |_: &usize| CasDecision::finish(())));
    let _ = result.expect("rich finish");
    let replacement = Arc::new(1);
    let (result, update_arc) = allocations(|| {
        executor.execute_result(&state, |_: &usize| {
            CasDecision::update_arc(Arc::clone(&replacement), ())
        })
    });
    result.expect("preallocated update");
    let (result, update) = allocations(|| executor.execute_result(&state, |_: &usize| CasDecision::update(2, ())));
    result.expect("owned update");
    assert_eq!(warm, 0, "warm finish does not allocate executor bookkeeping");
    assert_eq!(update_arc, 0, "preallocated update does not allocate bookkeeping");
    assert!(rich > warm, "report construction must have a separately measured cost");
    assert_eq!(
        update,
        update_arc + 1,
        "owned updates allocate exactly their additional Arc"
    );
    println!(
        "sync,constructor={constructor},cold_finish={cold},warm_finish={warm},rich_finish={rich},update_arc={update_arc},update={update}"
    );

    let replacement = Arc::new(3);
    let (result, rich_update_arc) = allocations(|| {
        executor.execute(&state, |_: &usize| {
            CasDecision::update_arc(Arc::clone(&replacement), ())
        })
    });
    let _ = result.expect("rich preallocated update");
    let (result, rich_update) = allocations(|| executor.execute(&state, |_: &usize| CasDecision::update(4, ())));
    let _ = result.expect("rich owned update");
    assert_eq!(rich_update, rich_update_arc + 1);
    println!("sync,rich_update_arc={rich_update_arc},rich_update={rich_update}");

    let attempts = Cell::new(0);
    let (result, conflict) = allocations(|| {
        executor.execute_result(&state, |current: &usize| {
            let attempt = attempts.get();
            attempts.set(attempt + 1);
            if attempt == 0 {
                state.store(Arc::new(*current + 1));
            }
            CasDecision::update(*current + 1, ())
        })
    });
    assert_eq!(result.expect("conflict retry").attempts(), 2);
    println!("sync,one_conflict={conflict}");
}

#[cfg(feature = "tokio")]
#[test]
fn test_async_allocation_measurements() {
    use std::future::Future;
    use std::task::Context;
    use std::task::Poll;
    use std::task::Waker;
    use std::time::Duration;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime");
    let _entered = runtime.enter();
    let mut context = Context::from_waker(Waker::noop());
    for timeout in [false, true] {
        let builder = CasExecutor::<usize, ()>::builder();
        let executor = if timeout {
            builder.attempt_timeout(Some(Duration::from_secs(1)))
        } else {
            builder
        }
        .build()
        .expect("valid timeouts");
        let state = AtomicRef::from_value(0usize);
        executor
            .execute_result(&state, |_: &usize| CasDecision::finish(()))
            .expect("initialize retry cache");
        let mut counts = Vec::new();
        for _ in 0..2 {
            let mut future =
                std::pin::pin!(executor.execute_async_result(&state, |_| async { CasDecision::finish(()) }));
            let (result, count) = allocations(|| future.as_mut().poll(&mut context));
            assert!(matches!(result, Poll::Ready(Ok(_))), "finish must be immediately ready");
            counts.push(count);
        }
        assert!(
            counts[1] <= if timeout { 6 } else { 0 },
            "snapshot bookkeeping must not add a heap allocation"
        );
        println!("async,timeout={timeout},first={},warm={}", counts[0], counts[1]);
    }
}
