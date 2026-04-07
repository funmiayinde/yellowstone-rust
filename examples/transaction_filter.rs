//! Stream successful non-vote transactions touching Kamino.
//!
//! cargo run --example transaction_filter

use futures::StreamExt;
use yellowstone_rust::{Update, YellowstoneClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = YellowstoneClient::builder()
        .endpoint(std::env::var("YELLOWSTONE_ENDPOINT").expect("required"))
        .connect();

    let kamino: solana_sdk::pubkey::Pubkey =
        "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD".parse().unwrap();

    let mut stream = client
        .subscribe()
        .name("kamino-txns")
        .transactions()
        .successful_only()
        .exclude_votes()
        .mentions_account(kamino)
        .build()
        .await?;

    println!("streaming Kamino transactions…");

    while let Some(update) = stream.next().await {
        if let Ok(Update::Transaction(tx)) = update {
            println!(
                "slot={:<10} sig={}  accounts={}",
                tx.slot,
                tx.signature_str(),
                tx.account_keys.len(),
            );
        }
    }

    Ok(())
}
