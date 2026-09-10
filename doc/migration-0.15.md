# Migrating to qubit-cas 0.15

[中文版](migration-0.15.zh_CN.md). This page applies to 0.15.

Version 0.15 intentionally makes breaking public-contract changes. It provides
no deprecated aliases, compatibility features, or forwarding bridges.

## Default business error

`CasExecutor<T>` and `CasBuilder<T>` now use `CasBoxError` as their default
business-error parameter. It wraps `qubit_error::BoxError`, implements
`std::error::Error`, and retains the wrapped error as `source()`. A default
terminal `CasError<T, CasBoxError>` can therefore participate in an ordinary
error source chain.

| Previous usage | 0.15 usage |
| --- | --- |
| Infer or name the default error as `BoxError` | Infer or name it as `CasBoxError` |
| Pass a concrete error through a generic conversion | `CasBoxError::new(Box::new(error))` |
| Match a default terminal error as `CasError<T, BoxError>` | Match it as `CasError<T, CasBoxError>` |

There is deliberately no blanket `impl<E: Error> From<E> for CasBoxError`.
Such an implementation overlaps with Rust's `From<T> for T` implementation.
Box the concrete error at the boundary instead. The wrapper's `as_inner()` and
`into_inner()` methods expose the retained boxed error when an integration needs
it.

## Clone bounds

Cloning `CasDecision`, `CasAttemptFailure`, `CasSuccess`, `CasError`, or
`CasOutcome` no longer requires `T: Clone`. These types share their `Arc<T>`
snapshots. Their owned output (`R`) or business error (`E`) still requires
`Clone` when that type itself is cloned. This is a bound relaxation, so callers
may remove unnecessary `T: Clone` constraints without changing behavior.

## Listener panic containment

In unwind builds, listener panic isolation now also contains a second panic from
the first panic payload's destructor. The confirmed CAS result and final report
remain available; the listener failure is recorded as a diagnostic. This does
not catch operation panics, and it cannot apply to `panic = "abort"` builds.

## Snapshot identity

CAS compares `Arc` identity with `Arc::ptr_eq`, never `T` value equality. Equal
values in distinct allocations are different snapshots. Derive an `update_arc`
replacement from the operation's current observation; do not use a historical
`Arc` as an A-B-A detector. Mutations behind `Mutex`, `Cell`, atomics, or other
interior mutability are outside the identity check. Model state immutably and
with an explicit version when those changes must be coordinated. Finally,
`CasSuccess::current()` is the successful attempt's published or observed
snapshot, not a guarantee that it remains the globally current value on return.

See the [user guide](user_guide.md), [design](design.md), and the historical
[0.14 migration note](migration-0.14.md).
