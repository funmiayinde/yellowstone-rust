//! Graceful shutdown on SIGINT.
//!
//! cargo run --example graceful_shutdown

use futures::StreamExt;
use yellowstone_rust::{Update, YellowstoneClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let client = YellowstoneClient::builder()
        .endpoint(std::env::var("YELLOWSTONE_ENDPOINT").expect("required"))
        .connect();

    let mut stream = client.subscribe().slots().build().await?;

    let mut sigint = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::interrupt()
    )?;

    tracing::info!("running, Ctrl+C to stop");

    loop {
        tokio::select! {
            _ = sigint.recv() => {
                tracing::info!("SIGINT received — shutting down");
                stream.cancel();
                break;
            }
            Some(update) = stream.next() => {
                if let Ok(Update::Slot(s)) = update {
                    println!("slot={} {:?}", s.slot, s.status);
                }
            }
        }
    }

    Ok(())
}
