use std::sync::Arc;

use crate::{
    backoff::{BackoffStrategy, ExponentialBackoff},
    builder::SubscriptionBuilder,
    config::ClientConfig,
    error::YellowstoneError,
};

// ── YellowstoneClient ─────────────────────────────────────────────────────

/// Entry point. Cheap to clone — all state is Arc-shared.
///
/// The gRPC connection is opened lazily inside the receive loop on first
/// `subscribe().build()`. Use `connect_and_validate()` for eager validation.
#[derive(Clone)]
pub struct YellowstoneClient {
    pub(crate) config:  Arc<ClientConfig>,
    pub(crate) backoff: Arc<dyn BackoffStrategy>,
}

impl YellowstoneClient {
    pub fn builder() -> ClientBuilder { ClientBuilder::default() }

    pub fn subscribe(&self) -> SubscriptionBuilder<'_> { SubscriptionBuilder::new(self) }

    pub fn config(&self) -> &ClientConfig { &self.config }
}

// ── ClientBuilder ─────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ClientBuilder {
    config:  ClientConfig,
    backoff: Option<Box<dyn BackoffStrategy>>,
}

impl ClientBuilder {
    pub fn endpoint(mut self, ep: impl Into<String>) -> Self {
        self.config.endpoint = ep.into();
        self
    }

    pub fn auth_token(mut self, token: impl Into<String>) -> Self {
        self.config.auth_token = Some(token.into());
        self
    }

    pub fn config(mut self, config: ClientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn backoff(mut self, b: impl BackoffStrategy + 'static) -> Self {
        self.backoff = Some(Box::new(b));
        self
    }

    pub fn buffer_size(mut self, n: usize) -> Self {
        self.config.buffer_size = n;
        self
    }

    pub fn slow_consumer_policy(mut self, p: crate::config::SlowConsumerPolicy) -> Self {
        self.config.slow_consumer_policy = p;
        self
    }

    /// Apply an optional value without breaking the chain.
    pub fn apply_opt<T, F>(self, value: Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, T) -> Self,
    {
        match value { Some(v) => f(self, v), None => self }
    }

    /// Build the client synchronously.  The gRPC connection opens lazily on
    /// the first `subscribe().build().await`.
    pub fn connect(self) -> YellowstoneClient {
        assert!(!self.config.endpoint.is_empty(), "endpoint must not be empty");
        let backoff: Arc<dyn BackoffStrategy> = match self.backoff {
            Some(b) => Arc::from(b),
            None    => Arc::new(ExponentialBackoff::default()),
        };
        YellowstoneClient { config: Arc::new(self.config), backoff }
    }

    /// Async variant: validates the endpoint is reachable before returning.
    pub async fn connect_and_validate(self) -> Result<YellowstoneClient, YellowstoneError> {
        let client = self.connect();

        // Probe-connect to fail fast if the endpoint is unreachable.
        let builder = yellowstone_grpc_client::GeyserGrpcClient::build_from_shared(
            client.config.endpoint.clone()
        ).map_err(|e| YellowstoneError::Config(format!("invalid endpoint: {e}")))?;

        let builder = builder
            .x_token(client.config.auth_token.clone())
            .map_err(|e| YellowstoneError::Config(format!("invalid auth token: {e}")))?;

        builder
            .connect()
            .await
            .map_err(|e| YellowstoneError::Config(format!("endpoint unreachable: {e}")))?;

        #[cfg(feature = "tracing")]
        tracing::info!(endpoint = %client.config.endpoint, "yellowstone-rust: validated");

        Ok(client)
    }
}
