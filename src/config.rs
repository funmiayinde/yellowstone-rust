use std::path::PathBuf;
use std::time::Duration;

/// Full client configuration.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClientConfig {
    /// gRPC endpoint URI, e.g. "http://your-node:10000" or "https://node.com:443"
    pub endpoint: String,

    /// Bearer auth token sent as `x-token` gRPC metadata on every request.
    pub auth_token: Option<String>,

    /// Timeout for the initial connection attempt. Default: 10s
    #[cfg_attr(feature = "serde", serde(with = "duration_secs"))]
    pub connect_timeout: Duration,

    /// Timeout for the subscribe RPC call (not the stream). Default: 5s
    #[cfg_attr(feature = "serde", serde(with = "duration_secs"))]
    pub request_timeout: Duration,

    /// mpsc channel capacity per subscription. Default: 10_000
    pub buffer_size: usize,

    /// Policy when the consumer is slower than the gRPC stream. Default: DropOldest
    pub slow_consumer_policy: SlowConsumerPolicy,

    /// Max reconnect attempts before the stream errors permanently.
    /// `None` = retry forever.
    pub max_reconnect_attempts: Option<u32>,

    /// HTTP/2 keep-alive ping interval. Default: 20s
    #[cfg_attr(feature = "serde", serde(with = "duration_secs"))]
    pub http2_keep_alive_interval: Duration,

    /// HTTP/2 keep-alive ping timeout. Default: 5s
    #[cfg_attr(feature = "serde", serde(with = "duration_secs"))]
    pub http2_keep_alive_timeout: Duration,

    /// Maximum gRPC receive message size. Default: 64 MiB
    pub max_decoding_message_size: usize,

    /// TLS configuration. `None` = plaintext.
    pub tls: Option<TlsConfig>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint:                  "http://127.0.0.1:10000".into(),
            auth_token:                None,
            connect_timeout:           Duration::from_secs(10),
            request_timeout:           Duration::from_secs(5),
            buffer_size:               10_000,
            slow_consumer_policy:      SlowConsumerPolicy::DropOldest,
            max_reconnect_attempts:    None,
            http2_keep_alive_interval: Duration::from_secs(20),
            http2_keep_alive_timeout:  Duration::from_secs(5),
            max_decoding_message_size: 64 * 1024 * 1024,
            tls:                       None,
        }
    }
}

/// Behaviour when the consumer mpsc channel is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SlowConsumerPolicy {
    /// Block the receive loop — natural TCP backpressure to the gRPC server.
    Block,
    /// Drop the oldest buffered item and enqueue the incoming one.
    /// Best for stateful consumers (liquidation monitors, health trackers).
    #[default]
    DropOldest,
    /// Drop the incoming item. Buffer stays stable.
    DropNewest,
}

/// TLS configuration for the gRPC channel.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TlsConfig {
    /// Path to a custom CA certificate (PEM). Required for self-signed endpoints.
    pub ca_cert_path: Option<PathBuf>,
    /// Client certificate path (PEM) for mutual TLS.
    pub client_cert_path: Option<PathBuf>,
    /// Client private key path (PEM) for mutual TLS.
    pub client_key_path: Option<PathBuf>,
    /// Override the expected server domain name for TLS SNI.
    pub domain_override: Option<String>,
}

// ── TOML / serde helpers ──────────────────────────────────────────────────

#[cfg(feature = "serde")]
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_secs_f64().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_secs_f64(f64::deserialize(d)?))
    }
}
