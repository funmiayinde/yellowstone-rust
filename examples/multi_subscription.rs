//! Run Kamino + MarginFi subscriptions concurrently over one client.
//!
//! cargo run --example multi_subscription

use futures::StreamExt;
use yellowstone_rust::{Update, YellowstoneClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let client = YellowstoneClient::builder()
        .endpoint(std::env::var("YELLOWSTONE_ENDPOINT").expect("required"))
        .apply_opt(std::env::var("YELLOWSTONE_TOKEN").ok(), |b, t| b.auth_token(t))
        .connect();

    let kamino:   solana_sdk::pubkey::Pubkey =
        "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD".parse().unwrap();
    let marginfi: solana_sdk::pubkey::Pubkey =
        "MFv2hWf31Z9kbCa1snEPdcgp7oSMJKEiADEoxkFQoBo".parse().unwrap();

    // Two subscriptions, one client, one underlying connection.
    let mut kamino_stream = client
        .subscribe()
        .name("kamino")
        .accounts()
        .owner(kamino)
        .build()
        .await?;

    let mut marginfi_stream = client
        .subscribe()
        .name("marginfi")
        .accounts()
        .owner(marginfi)
        .build()
        .await?;

    loop {
        tokio::select! {
            Some(update) = kamino_stream.next() => {
                if let Ok(Update::Account(acc)) = update {
                    println!("[kamino]   {} @ slot {}", acc.pubkey, acc.slot);
                }
            }
            Some(update) = marginfi_stream.next() => {
                if let Ok(Update::Account(acc)) = update {
                    println!("[marginfi] {} @ slot {}", acc.pubkey, acc.slot);
                }
            }
        }
    }
}
