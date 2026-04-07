# Changelog

All notable changes to `yellowstone-rust` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [1.0.0] — 2025-01-01

### Added

- `YellowstoneClient` and `ClientBuilder` — full connection management
- `SubscriptionBuilder` with `AccountFilterBuilder`, `TransactionFilterBuilder`, `SlotFilterBuilder`
- `UpdateStream` — `futures::Stream<Item = Result<Update, StreamError>>` with transparent reconnect
- `Update` enum — `Account(AccountUpdate)`, `Transaction(TransactionUpdate)`, `Slot(SlotUpdate)`
- `AccountUpdate` with `Bytes`-backed data for zero-cost cloning
- `BackoffStrategy` trait with `ExponentialBackoff` (full jitter), `ConstantBackoff`, `ImmediateReconnect`
- `SlowConsumerPolicy` — `Block`, `DropOldest`, `DropNewest`
- `ClientConfig` with full `serde` support behind feature flag
- `TlsConfig` for custom CA certs and mutual TLS
- `TCP_NODELAY` applied by default on all channels
- Prometheus metrics behind `metrics` feature flag
- `tracing` instrumentation behind `tracing` feature flag
- 6 examples: `account_stream`, `multi_subscription`, `custom_backoff`, `graceful_shutdown`, `transaction_filter`, `metrics_server`
- Integration test suite covering backoff, config, update types, filter builders
- Criterion benchmarks: `decode_throughput`, `channel_throughput`
- GitHub Actions CI: check, clippy, fmt, test, feature matrix, MSRV 1.75, audit, publish
