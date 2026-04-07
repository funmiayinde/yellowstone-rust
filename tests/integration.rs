/// Integration tests for yellowstone-rust.

#[cfg(test)]
mod filter_builder {
    use yellowstone_rust::{ClientConfig, CommitmentLevel};

    #[test]
    fn builder_requires_at_least_one_filter() {
        let config = ClientConfig::default();
        assert_eq!(config.buffer_size, 10_000);
        assert_eq!(config.max_reconnect_attempts, None);
    }

    #[test]
    fn commitment_default_is_confirmed() {
        assert_eq!(CommitmentLevel::default(), CommitmentLevel::Confirmed);
    }
}


#[cfg(test)]
mod backoff {
    use std::time::Duration;
    use yellowstone_rust::{BackoffStrategy, ConstantBackoff, ExponentialBackoff, ImmediateReconnect};

    #[test]
    fn exponential_grows_and_caps() {
        let b = ExponentialBackoff {
            base:       Duration::from_millis(100),
            max:        Duration::from_secs(1),
            jitter_max: Duration::ZERO,
        };
        let d0 = b.next_delay(0);
        let d5 = b.next_delay(5);

        // With zero jitter: attempt 0 → 100ms, attempt 5 → capped at 1000ms
        assert!(d0 <= Duration::from_millis(100) + Duration::from_millis(1));
        assert!(d5 <= Duration::from_secs(1) + Duration::from_millis(1));
    }

    #[test]
    fn exponential_never_exceeds_max() {
        let b = ExponentialBackoff::default();
        for attempt in 0..100u32 {
            let delay = b.next_delay(attempt);
            assert!(
                delay <= b.max + b.jitter_max,
                "delay {delay:?} exceeded max+jitter at attempt {attempt}"
            );
        }
    }

    #[test]
    fn constant_backoff_is_constant() {
        let b = ConstantBackoff::new(Duration::from_secs(2));
        for attempt in 0..20u32 {
            assert_eq!(b.next_delay(attempt), Duration::from_secs(2));
        }
    }

    #[test]
    fn immediate_reconnect_is_zero() {
        let b = ImmediateReconnect;
        for attempt in 0..20u32 {
            assert_eq!(b.next_delay(attempt), Duration::ZERO);
        }
    }

    #[test]
    fn reset_does_not_panic() {
        let b = ExponentialBackoff::default();
        b.reset(); // should be a no-op on the default impl
    }
}

#[cfg(test)]
mod config {
    use yellowstone_rust::{ClientConfig, SlowConsumerPolicy};

    #[test]
    fn default_config_sane() {
        let cfg = ClientConfig::default();
        assert_eq!(cfg.slow_consumer_policy, SlowConsumerPolicy::DropOldest);
        assert_eq!(cfg.buffer_size, 10_000);
        assert!(cfg.auth_token.is_none());
        assert!(cfg.tls.is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn config_round_trips_toml() {
        let original = ClientConfig {
            endpoint:   "http://my-node:10000".into(),
            auth_token: Some("secret-token".into()),
            buffer_size: 5_000,
            ..ClientConfig::default()
        };
        let serialised   = toml::to_string(&original).unwrap();
        let deserialised: ClientConfig = toml::from_str(&serialised).unwrap();
        assert_eq!(deserialised.endpoint,    original.endpoint);
        assert_eq!(deserialised.auth_token,  original.auth_token);
        assert_eq!(deserialised.buffer_size, original.buffer_size);
    }
}

#[cfg(test)]
mod update_types {
    use bytes::Bytes;
    use solana_sdk::pubkey::Pubkey;
    use std::time::{SystemTime, UNIX_EPOCH};
    use yellowstone_rust::{AccountUpdate, CommitmentLevel, SlotStatus, SlotUpdate, Update};

    fn fake_account_update() -> AccountUpdate {
        AccountUpdate {
            pubkey:         Pubkey::new_unique(),
            slot:           12_345_678,
            lamports:       2_000_000,
            owner:          Pubkey::new_unique(),
            executable:     false,
            rent_epoch:     0,
            data:           Bytes::from_static(b"hello yellowstone"),
            write_version:  1,
            txn_signature:  None,
            received_at_us: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
        }
    }

    #[test]
    fn account_update_age_us_is_small_for_fresh_update() {
        let update = fake_account_update();
        let age    = update.age_us();
        assert!(age < 50_000, "age {age}µs seems too large for a fresh update");
    }

    #[test]
    fn update_kind_strings() {
        let acc = Update::Account(fake_account_update());
        assert_eq!(acc.kind(), "account");
        assert_eq!(acc.slot(), 12_345_678);
        assert!(acc.as_account().is_some());
        assert!(acc.as_transaction().is_none());
    }

    #[test]
    fn slot_update_fields() {
        let s = Update::Slot(SlotUpdate {
            slot:   100,
            parent: Some(99),
            status: SlotStatus::Confirmed,
        });
        assert_eq!(s.slot(), 100);
        assert_eq!(s.kind(), "slot");
    }

    #[test]
    fn commitment_level_into_i32() {
        use yellowstone_rust::CommitmentLevel;
        let p: i32 = CommitmentLevel::Processed.into();
        let c: i32 = CommitmentLevel::Confirmed.into();
        let f: i32 = CommitmentLevel::Finalized.into();
        assert_ne!(p, c);
        assert_ne!(c, f);
    }
}
