# Qubit CAS

[![Rust CI](https://github.com/qubit-ltd/rs-cas/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-cas/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-cas/coverage-badge.json)](https://qubit-ltd.github.io/rs-cas/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-cas.svg?color=blue)](https://crates.io/crates/qubit-cas)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

## 概览

`qubit-cas` 是面向不可变 `Arc<T>` 快照的强类型 compare-and-swap 执行器。
它把原子更新、面向竞争的重试、业务决策、可选超时和结构化报告组合到可复用的
`CasExecutor` 中。

当并发写入不能丢失，且一次操作可以从最新快照重新执行时，适合使用本 crate。
请保持 operation 确定，并让副作用具备幂等性；CAS 可能调用它多次。

## 安装

```toml
[dependencies]
qubit-cas = "0.12"
qubit-atomic = "0.13"
```

使用 `features = ["tokio"]` 启用异步执行。高级重试配置可通过
[`qubit_cas::retry`](https://docs.rs/qubit-cas/latest/qubit_cas/retry/index.html) 使用。

## 快速开始

```rust
use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasExecutor};

#[derive(Debug, PartialEq, Eq)]
struct Inventory { stock: u32 }

#[derive(Debug, Clone, PartialEq, Eq)]
enum OrderError { OutOfStock }

fn main() {
    let state = AtomicRef::from_value(Inventory { stock: 3 });
    let executor = CasExecutor::<Inventory, OrderError>::latency_first();
    let result = executor.execute_result(&state, |current: &Inventory| {
        if current.stock == 0 { return CasDecision::abort(OrderError::OutOfStock); }
        CasDecision::update(Inventory { stock: current.stock - 1 }, current.stock - 1)
    });
    let success = result.expect("stock should be available");
    assert_eq!(*success.output(), 2);
    assert_eq!(state.load().stock, 2);
}
```

| 决策 | 效果 |
| --- | --- |
| `update(next, output)` | 通过 CAS 安装新快照并返回输出。 |
| `finish(output)` | 不写入新快照，直接成功。 |
| `retry(error)` | 按配置的策略重试业务失败。 |
| `abort(error)` | 立即停止并返回分类后的业务错误。 |

## 选择执行路径

| 需求 | API |
| --- | --- |
| 只需要结果，不需要报告或 hooks | `execute_result` / `execute_async_result` |
| 需要尝试次数、冲突、耗时或报告 | `execute` / `execute_async` |
| 需要单次执行的事件或告警 | `execute_with_hooks` / `execute_async_with_hooks` |
| 数值型、无分配热路径 | [`qubit-fast-cas`](https://crates.io/crates/qubit-fast-cas) |

请阅读[用户指南](doc/user_guide.zh_CN.md)、[设计文档](doc/design.zh_CN.md)和
[0.11 迁移说明](doc/migration-0.11.zh_CN.md)。完整 Rustdoc 见
[API 文档](https://docs.rs/qubit-cas)。`qubit-fast-cas` 是独立的紧凑 `u64` 状态机 crate，
不提供报告、hooks、异步执行或业务重试。

## 测试

```bash
cargo test
cargo test --all-features
./ci-check.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献。请保持公共 API 文档与测试同步，提交 Pull Request 前运行 `./align-ci.sh`
和 `./ci-check.sh`。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-cas](https://github.com/qubit-ltd/rs-cas)
