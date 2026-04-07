use std::sync::Arc;
use solana_sdk::pubkey::Pubkey;
use tokio_util::sync::CancellationToken;

use yellowstone_grpc_proto::prelude::{
    CommitmentLevel as ProtoCommitment,
    SubscribeRequest,
    SubscribeRequestFilterAccounts,
    SubscribeRequestFilterAccountsFilter,
    SubscribeRequestFilterAccountsFilterMemcmp,
    SubscribeRequestFilterSlots,
    SubscribeRequestFilterTransactions,
    SubscribeRequestPing,
    subscribe_request_filter_accounts_filter::Filter as AccountFilter,
    subscribe_request_filter_accounts_filter_memcmp::Data as MemcmpData,
};

use crate::{
    client::YellowstoneClient,
    error::YellowstoneError,
    stream::{spawn_receive_loop, UpdateStream},
    types::CommitmentLevel,
};

// ── SubscriptionBuilder ───────────────────────────────────────────────────

pub struct SubscriptionBuilder<'a> {
    client:         &'a YellowstoneClient,
    account_filter: Option<SubscribeRequestFilterAccounts>,
    tx_filter:      Option<SubscribeRequestFilterTransactions>,
    slot_filter:    Option<SubscribeRequestFilterSlots>,
    commitment:     CommitmentLevel,
    name:           String,
}

impl<'a> SubscriptionBuilder<'a> {
    pub(crate) fn new(client: &'a YellowstoneClient) -> Self {
        Self {
            client,
            account_filter: None,
            tx_filter:      None,
            slot_filter:    None,
            commitment:     CommitmentLevel::default(),
            name:           "default".into(),
        }
    }

    pub fn accounts(self) -> AccountFilterBuilder<'a> {
        AccountFilterBuilder { parent: self, pubkeys: vec![], owners: vec![], filters: vec![] }
    }

    pub fn transactions(self) -> TransactionFilterBuilder<'a> {
        TransactionFilterBuilder {
            parent:          self,
            account_include: vec![],
            account_exclude: vec![],
            vote:            None,
            failed:          None,
            signature:       None,
        }
    }

    pub fn slots(mut self) -> SlotFilterBuilder<'a> {
        // Use ..Default::default() so any new proto fields (e.g. interslot_updates)
        // are zero-initialised automatically — resilient to proto version changes.
        self.slot_filter = Some(SubscribeRequestFilterSlots {
            filter_by_commitment: Some(true),
            ..Default::default()
        });
        SlotFilterBuilder { parent: self }
    }

    pub fn commitment(mut self, level: CommitmentLevel) -> Self {
        self.commitment = level;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub async fn build(self) -> Result<UpdateStream, YellowstoneError> {
        if self.account_filter.is_none()
            && self.tx_filter.is_none()
            && self.slot_filter.is_none()
        {
            return Err(YellowstoneError::config(
                "at least one filter (accounts / transactions / slots) must be set",
            ));
        }

        let commitment: i32 = self.commitment.into();
        let mut request = SubscribeRequest {
            commitment: Some(commitment),
            ping:       Some(SubscribeRequestPing { id: 1 }),
            ..Default::default()
        };

        if let Some(af) = self.account_filter {
            request.accounts.insert(self.name.clone(), af);
        }
        if let Some(tf) = self.tx_filter {
            request.transactions.insert(self.name.clone(), tf);
        }
        if let Some(sf) = self.slot_filter {
            request.slots.insert(self.name.clone(), sf);
        }

        let cancel = CancellationToken::new();
        let stream = spawn_receive_loop(
            self.client.config.clone(),
            self.client.backoff.clone(),
            request,
            self.name.clone(),
            cancel.clone(),
        );

        Ok(stream)
    }
}

// ── AccountFilterBuilder ──────────────────────────────────────────────────

pub struct AccountFilterBuilder<'a> {
    parent:  SubscriptionBuilder<'a>,
    pubkeys: Vec<String>,
    owners:  Vec<String>,
    filters: Vec<SubscribeRequestFilterAccountsFilter>,
}

impl<'a> AccountFilterBuilder<'a> {
    pub fn pubkey(mut self, pk: Pubkey) -> Self {
        self.pubkeys.push(pk.to_string());
        self
    }

    pub fn pubkeys(mut self, pks: impl IntoIterator<Item = Pubkey>) -> Self {
        self.pubkeys.extend(pks.into_iter().map(|p| p.to_string()));
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owners.push(owner.to_string());
        self
    }

    pub fn owners(mut self, owners: impl IntoIterator<Item = Pubkey>) -> Self {
        self.owners.extend(owners.into_iter().map(|o| o.to_string()));
        self
    }

    pub fn data_size(mut self, size: u64) -> Self {
        self.filters.push(SubscribeRequestFilterAccountsFilter {
            filter: Some(AccountFilter::Datasize(size)),
        });
        self
    }

    pub fn memcmp(mut self, offset: u64, bytes: impl Into<Vec<u8>>) -> Self {
        self.filters.push(SubscribeRequestFilterAccountsFilter {
            filter: Some(AccountFilter::Memcmp(
                SubscribeRequestFilterAccountsFilterMemcmp {
                    offset,
                    data: Some(MemcmpData::Bytes(bytes.into())),
                },
            )),
        });
        self
    }

    pub fn and(mut self) -> SubscriptionBuilder<'a> {
        self.parent.account_filter = Some(SubscribeRequestFilterAccounts {
            account: self.pubkeys,
            owner:   self.owners,
            filters: self.filters,
            // Forward-compatible: zero-init any new proto fields automatically.
            ..Default::default()
        });
        self.parent
    }

    pub async fn build(self) -> Result<UpdateStream, YellowstoneError> {
        self.and().build().await
    }
}

// ── TransactionFilterBuilder ──────────────────────────────────────────────

pub struct TransactionFilterBuilder<'a> {
    parent:          SubscriptionBuilder<'a>,
    account_include: Vec<String>,
    account_exclude: Vec<String>,
    vote:            Option<bool>,
    failed:          Option<bool>,
    signature:       Option<String>,
}

impl<'a> TransactionFilterBuilder<'a> {
    pub fn successful_only(mut self) -> Self { self.failed    = Some(false); self }
    pub fn failed_only(mut self)     -> Self { self.failed    = Some(true);  self }
    pub fn vote_only(mut self)       -> Self { self.vote      = Some(true);  self }
    pub fn exclude_votes(mut self)   -> Self { self.vote      = Some(false); self }

    pub fn mentions_account(mut self, pk: Pubkey) -> Self {
        self.account_include.push(pk.to_string());
        self
    }

    pub fn mentions_accounts(mut self, pks: impl IntoIterator<Item = Pubkey>) -> Self {
        self.account_include.extend(pks.into_iter().map(|p| p.to_string()));
        self
    }

    pub fn excludes_account(mut self, pk: Pubkey) -> Self {
        self.account_exclude.push(pk.to_string());
        self
    }

    pub fn signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = Some(sig.into());
        self
    }

    pub fn and(mut self) -> SubscriptionBuilder<'a> {
        self.parent.tx_filter = Some(SubscribeRequestFilterTransactions {
            vote:            self.vote,
            failed:          self.failed,
            account_include: self.account_include,
            account_exclude: self.account_exclude,
            signature:       self.signature,
            // Forward-compatible: zero-init any new proto fields (e.g. account_required,
            // account_mandatory, or any future additions) automatically.
            ..Default::default()
        });
        self.parent
    }

    pub async fn build(self) -> Result<UpdateStream, YellowstoneError> {
        self.and().build().await
    }
}

// ── SlotFilterBuilder ─────────────────────────────────────────────────────

pub struct SlotFilterBuilder<'a> {
    parent: SubscriptionBuilder<'a>,
}

impl<'a> SlotFilterBuilder<'a> {
    pub fn and(self) -> SubscriptionBuilder<'a> { self.parent }

    pub async fn build(self) -> Result<UpdateStream, YellowstoneError> {
        self.and().build().await
    }
}