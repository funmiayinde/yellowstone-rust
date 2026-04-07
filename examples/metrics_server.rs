//! Expose Prometheus metrics at :9090/metrics while streaming slots.
//!
//! cargo run --example metrics_server --features metrics

#[cfg(feature = "metrics")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::{Router, routing::get};
    use futures::StreamExt;
    use yellowstone_rust::{encode_metrics, Update, YellowstoneClient};

    tracing_subscriber::fmt().with_env_filter("info").init();

    // Spawn /metrics HTTP server on :9090
    let app = Router::new().route("/metrics", get(|| async {
        encode_metrics().unwrap_or_default()
    }));
    tokio::spawn(async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await.unwrap();
        tracing::info!("metrics available at http://0.0.0.0:9090/metrics");
        axum::serve(listener, app).await.unwrap();
    });

    let client = YellowstoneClient::builder()
        .endpoint(std::env::var("YELLOWSTONE_ENDPOINT").expect("required"))
        .connect();

    let mut stream = client
        .subscribe()
        .name("slot-monitor")
        .slots()
        .build()
        .await?;

    while let Some(update) = stream.next().await {
        if let Ok(Update::Slot(s)) = update {
            println!("slot={} {:?}  buffer={:.1}%",
                s.slot, s.status, stream.buffer_utilization() * 100.0);
        }
    }

    Ok(())
}

#[cfg(not(feature = "metrics"))]
fn main() {
    eprintln!("Build with --features metrics to run this example.");
}
