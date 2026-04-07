#![cfg(feature = "metrics")]

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, GaugeVec, HistogramVec,
};


pub(crate) static RECONNECTS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "yellowstone_reconnects_total",
        "Total number of stream reconnect events",
        &["subscription"]
    ).expect("metric registration")
});

pub(crate) static UPDATES_RECEIVED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "yellowstone_updates_received_total",
        "Total number of updates decoded and delivered to the caller",
        &["subscription", "update_type"]
    ).expect("metric registration")
});

pub(crate) static UPDATES_DROPPED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "yellowstone_updates_dropped_total",
        "Total number of updates dropped due to slow consumer",
        &["subscription"]
    ).expect("metric registration")
});

pub(crate) static CONSUMER_LAG: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "yellowstone_consumer_lag_ms",
        "Delay in milliseconds between gRPC receive and caller next()",
        &["subscription"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]
    ).expect("metric registration")
});

pub(crate) static BUFFER_UTILIZATION: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "yellowstone_buffer_utilization",
        "Current buffer fill ratio (0.0 to 1.0) per subscription",
        &["subscription"]
    ).expect("metric registration")
});

pub(crate) static CONNECTED: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "yellowstone_connected",
        "1 if the subscription is currently connected, 0 if reconnecting",
        &["subscription"]
    ).expect("metric registration")
});

pub(crate) static STREAM_UPTIME: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "yellowstone_stream_uptime_seconds",
        "Seconds since the last successful (re)connection",
        &["subscription"]
    ).expect("metric registration")
});

/// Encode all registered Prometheus metrics as a UTF-8 text exposition string.
/// Serve this at your `/metrics` HTTP endpoint.
pub fn encode_metrics() -> Result<String, prometheus::Error> {
    use prometheus::Encoder;
    let encoder  = prometheus::TextEncoder::new();
    let families = prometheus::gather();
    let mut buf  = Vec::new();
    encoder.encode(&families, &mut buf)?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub subscription:          String,
    pub reconnects_total:      f64,
    pub updates_received:      f64,
    pub updates_dropped:       f64,
    pub buffer_utilization:    f64,
    pub connected:             bool,
    pub stream_uptime_seconds: f64,
}
