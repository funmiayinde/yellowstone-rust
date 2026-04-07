//! # yellowstone-rust
//!
//! Production-grade, ultra-low latency Rust client for Yellowstone gRPC.
//!
//! Provides transparent auto-reconnect, typed update envelopes, ergonomic
//! subscription filters, configurable backpressure, and optional Prometheus
//! metrics — everything you need above the raw proto bindings.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use futures::StreamExt;
//! use yellowstone_rust::{YellowstoneClient, Update};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = YellowstoneClient::builder()
//!         .endpoint("http://your-node:10000")
//!         .auth_token("your-token")
//!         .connect(); // sync — connection opened lazily on first subscribe
//!
//!     let kamino = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD".parse().unwrap();
//!
//!     let mut stream = client
//!         .subscribe()
//!         .name("kamino-obligations")
//!         .accounts()
//!         .owner(kamino)
//!         .build()
//!         .await?;
//!
//!     while let Some(update) = stream.next().await {
//!         match update? {
//!             Update::Account(acc) => {
//!                 println!("{} @ slot {} ({} bytes)", acc.pubkey, acc.slot, acc.data.len());
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod backoff;
pub mod builder;
pub mod client;
pub mod config;
pub mod error;
pub mod stream;
pub mod types;

#[cfg(feature = "metrics")]
#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
pub mod metrics;

// ── rustls crypto provider ────────────────────────────────────────────────
//
// rustls 0.23 requires exactly one crypto backend to be registered before
// any TLS connection is opened.  Call this once at process startup — it is
// idempotent (the second call returns Err and can be ignored).

/// Install the ring crypto provider for rustls.  Call this once at the top
/// of `main()` before any Yellowstone connections are opened.
///
/// ```rust,no_run
/// fn main() {
///     yellowstone_rust::install_crypto_provider();
///     // ... rest of startup
/// }
/// ```
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}


pub use backoff::{BackoffStrategy, ConstantBackoff, ExponentialBackoff, ImmediateReconnect};
pub use builder::{AccountFilterBuilder, SlotFilterBuilder, SubscriptionBuilder, TransactionFilterBuilder};
pub use client::{ClientBuilder, YellowstoneClient};
pub use config::{ClientConfig, SlowConsumerPolicy, TlsConfig};
pub use error::{StreamError, YellowstoneError};
pub use stream::UpdateStream;
pub use types::{
    AccountUpdate, CommitmentLevel, SlotStatus, SlotUpdate,
    TokenBalance, TransactionUpdate, Update,
};

#[cfg(feature = "metrics")]
pub use metrics::{encode_metrics, MetricsSnapshot};
