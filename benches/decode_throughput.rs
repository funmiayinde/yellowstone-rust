use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use solana_sdk::pubkey::Pubkey;
use std::time::{SystemTime, UNIX_EPOCH};
use yellowstone_rust::AccountUpdate;

fn make_account_update(data_size: usize) -> AccountUpdate {
    AccountUpdate {
        pubkey:         Pubkey::new_unique(),
        slot:           100_000_000,
        lamports:       2_039_280,
        owner:          Pubkey::new_unique(),
        executable:     false,
        rent_epoch:     0,
        data:           Bytes::from(vec![0u8; data_size]),
        write_version:  42,
        txn_signature:  None,
        received_at_us: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
    }
}

fn bench_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("account_update_clone");

    for size in [165usize, 3_336, 65_536] {
        let update = make_account_update(size);
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &update, |b, u| {
            b.iter(|| {
                let _ = black_box(u.clone());
            })
        });
    }

    group.finish();
}

fn bench_age_us(c: &mut Criterion) {
    let update = make_account_update(165);
    c.bench_function("age_us", |b| {
        b.iter(|| black_box(update.age_us()))
    });
}

fn bench_bytes_subslice(c: &mut Criterion) {
    // Simulates reading a discriminator from account data — extremely hot path.
    let update = make_account_update(3_336);
    c.bench_function("discriminator_read", |b| {
        b.iter(|| {
            let disc = &update.data[..8];
            black_box(disc);
        })
    });
}

criterion_group!(decode, bench_clone, bench_age_us, bench_bytes_subslice);
criterion_main!(decode);
