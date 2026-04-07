//! Custom backoff example.
//!
//! cargo run --example custom_backoff

use std::time::Duration;
use futures::StreamExt;
use yellowstone_rust::{ConstantBackoff, Update, YellowstoneClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = YellowstoneClient::builder()
        .endpoint(std::env::var("YELLOWSTONE_ENDPOINT").expect("required"))
        // Reconnect every 2 seconds with no jitter — useful in controlled environments.
        .backoff(ConstantBackoff::new(Duration::from_secs(2)))
        .connect();

    let mut stream = client.subscribe().slots().build().await?;

    while let Some(update) = stream.next().await {
        if let Ok(Update::Slot(s)) = update {
            println!("slot={} {:?}", s.slot, s.status);
        }
    }

    Ok(())
}
