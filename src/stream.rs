use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::{SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof,
    SubscribeRequest,
};

use crate::{
    config::{ClientConfig, SlowConsumerPolicy},
    error::StreamError,
    types::{AccountUpdate, SlotStatus, SlotUpdate, TransactionUpdate, Update},
};

#[cfg(feature = "tracing")]
use tracing::{error, info, warn};

#[cfg(feature = "metrics")]
use crate::metrics::{CONNECTED, RECONNECTS, UPDATES_DROPPED, UPDATES_RECEIVED};


#[pin_project::pin_project]
pub struct UpdateStream {
    #[pin]
    rx:         mpsc::Receiver<Result<Update, StreamError>>,
    cancel:     CancellationToken,
    name:       String,
    buffer_cap: usize,
}

impl UpdateStream {
    pub(crate) fn new(
        rx:         mpsc::Receiver<Result<Update, StreamError>>,
        cancel:     CancellationToken,
        name:       String,
        buffer_cap: usize,
    ) -> Self {
        Self { rx, cancel, name, buffer_cap }
    }

    pub fn buffer_len(&self) -> usize       { self.rx.len() }
    pub fn buffer_utilization(&self) -> f64 { self.rx.len() as f64 / self.buffer_cap as f64 }
    pub fn cancel(self)                     { self.cancel.cancel(); }
    pub fn is_closed(&self) -> bool         { self.rx.is_closed() }
}

impl Stream for UpdateStream {
    type Item = Result<Update, StreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().rx.poll_recv(cx)
    }
}


pub(crate) fn spawn_receive_loop(
    config:  Arc<ClientConfig>,
    backoff: Arc<dyn crate::backoff::BackoffStrategy>,
    request: SubscribeRequest,
    name:    String,
    cancel:  CancellationToken,
) -> UpdateStream {
    let cap      = config.buffer_size;
    let (tx, rx) = mpsc::channel(cap);
    let policy   = config.slow_consumer_policy;

    tokio::spawn(receive_loop(config, backoff, request, name.clone(), cancel.clone(), tx, policy));
    UpdateStream::new(rx, cancel, name, cap)
}


async fn receive_loop(
    config:  Arc<ClientConfig>,
    backoff: Arc<dyn crate::backoff::BackoffStrategy>,
    request: SubscribeRequest,
    name:    String,
    cancel:  CancellationToken,
    tx:      mpsc::Sender<Result<Update, StreamError>>,
    policy:  SlowConsumerPolicy,
) {
    let mut attempt: u32 = 0;

    'reconnect: loop {
        if cancel.is_cancelled() { return; }

        if let Some(max) = config.max_reconnect_attempts {
            if attempt > max {
                let _ = tx.send(Err(StreamError::PermanentlyClosed { attempts: attempt })).await;
                return;
            }
        }

        if attempt > 0 {
            let delay = backoff.next_delay(attempt - 1);

            #[cfg(feature = "tracing")]
            warn!(subscription = %name, attempt, delay_ms = delay.as_millis(),
                  "yellowstone-rust: reconnecting");

            #[cfg(feature = "metrics")]
            RECONNECTS.with_label_values(&[&name]).inc();

            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = cancel.cancelled()        => return,
            }
        }
        
        let builder = match GeyserGrpcClient::build_from_shared(config.endpoint.clone()) {
            Ok(b)  => b,
            Err(e) => {
                #[cfg(feature = "tracing")]
                error!(error = %e, "yellowstone-rust: failed to build client");
                attempt += 1;
                continue 'reconnect;
            }
        };

        let builder = match builder.x_token(config.auth_token.clone()) {
            Ok(b)  => b,
            Err(e) => {
                #[cfg(feature = "tracing")]
                error!(error = ?e, "yellowstone-rust: invalid auth token");
                attempt += 1;
                continue 'reconnect;
            }
        };

        let tls = tonic::transport::ClientTlsConfig::new().with_native_roots();

        let builder = match builder.tls_config(tls) {
            Ok(b)  => b,
            Err(e) => {
                #[cfg(feature = "tracing")]
                error!(error = ?e, "yellowstone-rust: tls_config failed");
                attempt += 1;
                continue 'reconnect;
            }
        };

        let mut grpc = match builder.connect().await {
            Ok(c) => {
                backoff.reset();
                attempt = 0;
                #[cfg(feature = "tracing")]
                info!(subscription = %name, "yellowstone-rust: connected");
                #[cfg(feature = "metrics")]
                CONNECTED.with_label_values(&[&name]).set(1.0);
                c
            }
            Err(e) => {
                #[cfg(feature = "tracing")]
                error!(error = ?e, "yellowstone-rust: connect failed");
                #[cfg(feature = "metrics")]
                CONNECTED.with_label_values(&[&name]).set(0.0);
                attempt += 1;
                continue 'reconnect;
            }
        };

        let (mut sub_tx, mut stream) = match grpc.subscribe().await {
            Ok(pair) => pair,
            Err(e) => {
                #[cfg(feature = "tracing")]
                warn!(error = %e, "yellowstone-rust: subscribe call failed");
                #[cfg(feature = "metrics")]
                CONNECTED.with_label_values(&[&name]).set(0.0);
                attempt += 1;
                continue 'reconnect;
            }
        };

        if let Err(e) = sub_tx.send(request.clone()).await {
            #[cfg(feature = "tracing")]
            warn!(error = %e, "yellowstone-rust: failed to send subscribe request");
            attempt += 1;
            continue 'reconnect;
        }

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    #[cfg(feature = "metrics")]
                    CONNECTED.with_label_values(&[&name]).set(0.0);
                    return;
                }
                msg = stream.next() => match msg {
                    Some(Ok(msg)) => {
                        match decode_update(msg) {
                            Ok(Some(typed)) => {
                                #[cfg(feature = "metrics")]
                                UPDATES_RECEIVED.with_label_values(&[&name, typed.kind()]).inc();
                                emit(&tx, Ok(typed), policy, &name).await;
                            }
                            Ok(None) => {} // ping / heartbeat — discard
                            Err(e)   => { emit(&tx, Err(e), policy, &name).await; }
                        }
                    }
                    Some(Err(status)) => {
                        #[cfg(feature = "tracing")]
                        warn!(subscription = %name, code = ?status.code(),
                              msg = %status.message(), "yellowstone-rust: stream error");
                        #[cfg(feature = "metrics")]
                        CONNECTED.with_label_values(&[&name]).set(0.0);
                        attempt += 1;
                        continue 'reconnect;
                    }
                    None => {
                        #[cfg(feature = "tracing")]
                        warn!(subscription = %name, "yellowstone-rust: server closed stream");
                        #[cfg(feature = "metrics")]
                        CONNECTED.with_label_values(&[&name]).set(0.0);
                        attempt += 1;
                        continue 'reconnect;
                    }
                }
            }
        }
    }
}

// Slow-consumer emit

async fn emit(
    tx:     &mpsc::Sender<Result<Update, StreamError>>,
    item:   Result<Update, StreamError>,
    policy: SlowConsumerPolicy,
    name:   &str,
) {
    match policy {
        SlowConsumerPolicy::Block => { let _ = tx.send(item).await; }
        SlowConsumerPolicy::DropNewest | SlowConsumerPolicy::DropOldest => {
            if tx.try_send(item).is_err() {
                #[cfg(feature = "metrics")]
                UPDATES_DROPPED.with_label_values(&[name]).inc();
            }
        }
    }
}

// Proto decoder
fn decode_update(
    msg: yellowstone_grpc_proto::prelude::SubscribeUpdate,
) -> Result<Option<Update>, StreamError> {
    let Some(inner) = msg.update_oneof else { return Ok(None); };

    match inner {
        UpdateOneof::Account(a) => {
            let Some(info) = a.account else { return Ok(None); };
            Ok(Some(Update::Account(AccountUpdate::from_proto(info, a.slot)?)))
        }

        UpdateOneof::Transaction(t) => {
            Ok(Some(Update::Transaction(TransactionUpdate::from_proto(t)?)))
        }

        UpdateOneof::Slot(s) => {
            let status = match s.status {
                1 => SlotStatus::Confirmed,
                2 => SlotStatus::Finalized,
                _ => SlotStatus::Processed,
            };
            Ok(Some(Update::Slot(SlotUpdate {
                slot:   s.slot,
                parent: s.parent,
                status,
            })))
        }

        _ => Ok(None),
    }
}
