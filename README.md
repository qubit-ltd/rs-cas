# Qubit CAS

Typed compare-and-swap executor for Rust.

`qubit-cas` builds on `qubit-atomic`, `qubit-function`, and `qubit-retry` to
provide:

- typed CAS decisions (`update`, `finish`, `retry`, `abort`)
- compare-and-swap conflict retry with configurable backoff
- synchronous and asynchronous execution APIs
- per-execution hooks for success, retry, and abort observation
- preset builders for high-concurrency, low-latency, and high-reliability modes

## Status

This crate is initialized from the same project template style used by
`rs-retry` and is intended to stay consistent with that repository's layout,
tests, and CI scripts.
