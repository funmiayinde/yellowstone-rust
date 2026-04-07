# yellowstone-rust

[![Crates.io](https://img.shields.io/crates/v/yellowstone-rust.svg)](https://crates.io/crates/yellowstone-rust)
[![docs.rs](https://img.shields.io/docsrs/yellowstone-rust)](https://docs.rs/yellowstone-rust)
[![CI](https://github.com/sabidex/yellowstone-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/sabidex/yellowstone-rust/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Minimum Rust Version](https://img.shields.io/badge/rustc-1.75%2B-orange)](https://www.rust-lang.org)

**Production-grade, ultra-low latency Rust client for Yellowstone gRPC — the streaming backbone of real-time Solana applications.**

The official Yellowstone repository ships raw proto bindings. That is a protocol spec, not a client library. Every team building real-time Solana infrastructure in Rust ends up writing the same production scaffolding from scratch:

- Reconnection with exponential backoff and jitter
- Subscription filter construction (raw `SubscribeRequest` proto is painful)
- Backpressure handling when your consumer is slower than the stream
- Typed update envelopes — no more manual proto field matching
- Health and lag metrics

**yellowstone-rust** is that scaffolding, extracted, cleaned, hardened, and published as a first-class crate.

Built and maintained by [Sabitrade](https://sabitrade.xyz) — on-chain trading intelligence infrastructure for Solana.
Docs [Sabitrade Gitbook](https://funmiayinde11.gitbook.io/sabitrade)

---

## Features

| Feature | Description |
|---|---|
| **Typed update API** | `Update::Account`, `Update::Transaction`, `Update::Slot` — no raw proto unpacking |
| **Ergonomic filter builder** | Composable, compile-time checked subscription filters |
| **Auto-reconnect** | Transparent reconnection with configurable `BackoffStrategy` |
| **Backpressure** | Bounded async channel between gRPC receive loop and consumer |
| **Zero-copy account data** | `Bytes`-backed account data — clone is a pointer bump, not a memcpy |
| **TLS + auth** | Token auth and optional TLS, configurable per-client |
| **Prometheus metrics** | `reconnects_total`, `updates_received_total`, `consumer_lag_ms` (opt-in) |
| **Tracing integration** | `tracing` spans on every critical path |
| **Graceful shutdown** | `CancellationToken` propagated through the entire receive loop |

---

## Installation

```toml
[dependencies]
yellowstone-rust = "1.0"
tokio            = { version = "1", features = ["full"] }
futures          = "0.3"
```

### Feature flags

```toml
yellowstone-rust = { version = "1.0", features = ["metrics", "tls", "tracing", "serde"] }
```

| Feature | Default | Description |
|---|---|---|
| `metrics` | ✅ | Prometheus metrics via `prometheus` crate |
| `tls` | ✅ | TLS support via `rustls` |
| `tracing` | ✅ | `tracing` span instrumentation |
| `serde` | ❌ | `Serialize/Deserialize` on all public types |
| `compression` | ❌ | gRPC message compression (gzip) |

---

## Quick start

```rust
use futures::StreamExt;
use yellowstone_rust::{Update, YellowstoneClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = YellowstoneClient::builder()
        .endpoint("http://your-node:10000")
        .auth_token("your-token")
        .connect(); // sync

    let kamino = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD".parse().unwrap();

    let mut stream = client
        .subscribe()
        .name("kamino-obligations")
        .accounts()
        .owner(kamino)
        .build()
        .await?;

    while let Some(update) = stream.next().await {
        match update? {
            Update::Account(acc) => {
                println!("{} @ slot {} ({} bytes)", acc.pubkey, acc.slot, acc.data.len());
            }
            _ => {}
        }
    }

    Ok(())
}
```

---

## Multi-subscription (concurrent streams)

Two independent subscriptions over a single `YellowstoneClient`:

```rust
let mut kamino_stream = client
    .subscribe()
    .name("kamino")
    .accounts()
    .owner(kamino_program)
    .build()
    .await?;

let mut marginfi_stream = client
    .subscribe()
    .name("marginfi")
    .accounts()
    .owner(marginfi_program)
    .build()
    .await?;

loop {
    tokio::select! {
        Some(u) = kamino_stream.next()   => { /* handle kamino */ }
        Some(u) = marginfi_stream.next() => { /* handle marginfi */ }
    }
}
```

---

## Custom backoff

```rust
use std::time::Duration;
use yellowstone_rust::{BackoffStrategy, ConstantBackoff, YellowstoneClient};

let client = YellowstoneClient::builder()
    .endpoint("http://your-node:10000")
    .backoff(ConstantBackoff::new(Duration::from_secs(2)))
    .connect(); // sync
```

Implement `BackoffStrategy` for fully custom reconnection logic.

---

## Transaction filter

```rust
let mut stream = client
    .subscribe()
    .name("kamino-txns")
    .transactions()
    .successful_only()
    .exclude_votes()
    .mentions_account(kamino_program)
    .build()
    .await?;
```

---

## Prometheus metrics

```rust
// Serve at /metrics
use yellowstone_rust::encode_metrics;

let metrics_str = encode_metrics()?;
// respond with metrics_str from your HTTP handler
```

Available metrics:

| Metric | Type | Labels |
|---|---|---|
| `yellowstone_reconnects_total` | Counter | `subscription` |
| `yellowstone_updates_received_total` | Counter | `subscription`, `update_type` |
| `yellowstone_updates_dropped_total` | Counter | `subscription` |
| `yellowstone_consumer_lag_ms` | Histogram | `subscription` |
| `yellowstone_buffer_utilization` | Gauge | `subscription` |
| `yellowstone_connected` | Gauge | `subscription` |
| `yellowstone_stream_uptime_seconds` | Gauge | `subscription` |

---

## Latency tuning

- **`TCP_NODELAY`** is enabled by default, eliminating Nagle's algorithm delay.
- **`buffer_size`**: `10_000` suits most workloads. Reduce for memory-constrained environments.
- **`SlowConsumerPolicy::DropOldest`**: best for stateful consumers (liquidation monitors, health trackers) — always processes fresh state.
- **`CommitmentLevel::Confirmed`**: recommended for MEV and liquidation. Sub-millisecond lower latency than `Finalized`, 99.9%+ fork-safe.
- **Profile with `tokio-console`**: add `console-subscriber` to your dev dependencies and `tokio-console` binary for real-time task scheduling visibility.

---

## Running examples

```bash
export YELLOWSTONE_ENDPOINT=http://your-node:10000
export YELLOWSTONE_TOKEN=your-token

cargo run --example account_stream
cargo run --example multi_subscription
cargo run --example graceful_shutdown
cargo run --example transaction_filter
cargo run --example metrics_server --features metrics
```

---

## Running tests

```bash
# Unit + integration tests
cargo test --all-features

# Benchmarks (compile check)
cargo bench --all-features --no-run

# Run benchmarks
cargo bench --all-features
```

---

## License

MIT. See [LICENSE](LICENSE).

> **Built by [Sabitrade](https://sabitrade.xyz)**
> `yellowstone-rust` is the transport layer. For behavioral risk scoring, wallet clustering, and liquidation simulation on top of this stream, see [ChainIntel](https://sabitrade.xyz/chainintel).
