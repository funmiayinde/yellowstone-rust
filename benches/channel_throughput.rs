use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use solana_sdk::pubkey::Pubkey;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use yellowstone_rust::{AccountUpdate, Update};

fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

fn bench_mpsc_roundtrip(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let update = Update::Account(AccountUpdate {
        pubkey:         Pubkey::new_unique(),
        slot:           1,
        lamports:       1,
        owner:          Pubkey::new_unique(),
        executable:     false,
        rent_epoch:     0,
        data:           Bytes::from_static(b"bench"),
        write_version:  1,
        txn_signature:  None,
        received_at_us: now_us(),
    });

    let mut group = c.benchmark_group("mpsc_channel");
    group.throughput(Throughput::Elements(1));

    // Simulate the hot path: one send + one recv on the same thread.
    group.bench_function("send_recv_roundtrip", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (tx, mut rx) = mpsc::channel::<Update>(10_000);
                tx.send(black_box(update.clone())).await.unwrap();
                let _ = black_box(rx.recv().await);
            });
        })
    });

    group.finish();
}

fn bench_channel_try_send_full(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let update = Update::Account(AccountUpdate {
        pubkey:         Pubkey::new_unique(),
        slot:           1,
        lamports:       1,
        owner:          Pubkey::new_unique(),
        executable:     false,
        rent_epoch:     0,
        data:           Bytes::from_static(b"bench"),
        write_version:  1,
        txn_signature:  None,
        received_at_us: now_us(),
    });

    let mut group = c.benchmark_group("mpsc_try_send");
    group.throughput(Throughput::Elements(1));

    group.bench_function("try_send_on_non_full_channel", |b| {
        rt.block_on(async {
            let (tx, _rx) = mpsc::channel::<Update>(10_000);
            b.iter(|| {
                let _ = tx.try_send(black_box(update.clone()));
            });
        });
    });

    group.finish();
}

criterion_group!(channel, bench_mpsc_roundtrip, bench_channel_try_send_full);
criterion_main!(channel);
