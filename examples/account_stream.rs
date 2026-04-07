use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;
use yellowstone_rust::{Update, YellowstoneClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    yellowstone_rust::install_crypto_provider();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,yellowstone_rust=debug".into()),
        )
        .init();

    let client = YellowstoneClient::builder()
        .endpoint(std::env::var("YELLOWSTONE_ENDPOINT").expect("YELLOWSTONE_ENDPOINT required"))
        .apply_opt(std::env::var("YELLOWSTONE_TOKEN").ok(), |b, t| b.auth_token(t))
        .buffer_size(10_000)
        .connect();

    let kamino: Pubkey = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD".parse().unwrap();

    let mut stream = client
        .subscribe()
        .name("kamino-obligations")
        .accounts()
        .owner(kamino)
        .build()
        .await?;

    tracing::info!("streaming Kamino account updates…");

    while let Some(update) = stream.next().await {
        match update? {
            Update::Account(acc) => {
                println!(
                    "slot={:<10} pubkey={}  lamports={:<12} data={}B  age={}µs",
                    acc.slot,
                    acc.pubkey,
                    acc.lamports,
                    acc.data.len(),
                    acc.age_us(),
                );
            }
            _ => {}
        }
    }

    Ok(())
}