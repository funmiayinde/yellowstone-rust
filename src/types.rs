use bytes::Bytes;
use solana_sdk::pubkey::Pubkey;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub pubkey:         Pubkey,
    pub slot:           u64,
    pub lamports:       u64,
    pub owner:          Pubkey,
    pub executable:     bool,
    pub rent_epoch:     u64,
    pub data:           Bytes,
    pub write_version:  u64,
    pub txn_signature:  Option<[u8; 64]>,
    pub received_at_us: u64,
}

impl AccountUpdate {
    pub fn age_us(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        now.saturating_sub(self.received_at_us)
    }

    pub(crate) fn from_proto(
        info: yellowstone_grpc_proto::prelude::SubscribeUpdateAccountInfo,
        slot: u64,
    ) -> Result<Self, crate::error::StreamError> {
        let pubkey = parse_pubkey(&info.pubkey)?;
        let owner  = parse_pubkey(&info.owner)?;

        let txn_signature = info.txn_signature
            .as_deref()
            .filter(|s| s.len() == 64)
            .map(|s| {
                let mut arr = [0u8; 64];
                arr.copy_from_slice(s);
                arr
            });

        let received_at_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        Ok(Self {
            pubkey,
            slot,
            lamports:      info.lamports,
            owner,
            executable:    info.executable,
            rent_epoch:    info.rent_epoch,
            data:          Bytes::copy_from_slice(&info.data),
            write_version: info.write_version,
            txn_signature,
            received_at_us,
        })
    }
}


#[derive(Debug, Clone)]
pub struct TransactionUpdate {
    pub signature:           [u8; 64],
    pub slot:                u64,
    pub is_vote:             bool,
    pub succeeded:           bool,
    pub fee:                 u64,
    pub account_keys:        Vec<Pubkey>,
    pub log_messages:        Vec<String>,
    pub pre_token_balances:  Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
    pub raw:                 Bytes,
    pub received_at_us:      u64,
}

impl TransactionUpdate {
    pub fn signature_str(&self) -> String {
        bs58::encode(&self.signature).into_string()
    }

    pub(crate) fn from_proto(
        tx_msg: yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
    ) -> Result<Self, crate::error::StreamError> {
        let tx   = tx_msg.transaction.ok_or_else(|| crate::error::StreamError::decode("missing transaction"))?;
        let meta = tx.meta.ok_or_else(|| crate::error::StreamError::decode("missing meta"))?;
        let txn  = tx.transaction.ok_or_else(|| crate::error::StreamError::decode("missing txn body"))?;
        let msg  = txn.message.ok_or_else(|| crate::error::StreamError::decode("missing message"))?;

        let account_keys: Vec<Pubkey> = msg
            .account_keys
            .iter()
            .map(|k| parse_pubkey(k))
            .collect::<Result<_, _>>()?;

        let sig = if tx.signature.len() == 64 {
            let mut s = [0u8; 64];
            s.copy_from_slice(&tx.signature);
            s
        } else {
            [0u8; 64]
        };

        let received_at_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        Ok(Self {
            signature:           sig,
            slot:                tx_msg.slot,
            is_vote:             tx.is_vote,
            succeeded:           meta.err.is_none(),
            fee:                 meta.fee,
            account_keys,
            log_messages:        meta.log_messages,
            pre_token_balances:  vec![],
            post_token_balances: vec![],
            raw:                 Bytes::new(),
            received_at_us,
        })
    }
}

// SlotUpdate
#[derive(Debug, Clone, Copy)]
pub struct SlotUpdate {
    pub slot:   u64,
    pub parent: Option<u64>,
    pub status: SlotStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SlotStatus {
    Processed,
    Confirmed,
    Finalized,
}

// TokenBalance

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub account_index: u32,
    pub mint:          Pubkey,
    pub owner:         Pubkey,
    pub amount:        u64,
    pub decimals:      u8,
}

// CommitmentLevel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CommitmentLevel {
    Processed,
    #[default]
    Confirmed,
    Finalized,
}

impl From<CommitmentLevel> for i32 {
    fn from(c: CommitmentLevel) -> Self {
        use yellowstone_grpc_proto::prelude::CommitmentLevel as P;
        match c {
            CommitmentLevel::Processed => P::Processed as i32,
            CommitmentLevel::Confirmed => P::Confirmed as i32,
            CommitmentLevel::Finalized => P::Finalized as i32,
        }
    }
}

// Update
#[derive(Debug, Clone)]
pub enum Update {
    Account(AccountUpdate),
    Transaction(TransactionUpdate),
    Slot(SlotUpdate),
}

impl Update {
    pub fn slot(&self) -> u64 {
        match self {
            Update::Account(a)     => a.slot,
            Update::Transaction(t) => t.slot,
            Update::Slot(s)        => s.slot,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Update::Account(_)     => "account",
            Update::Transaction(_) => "transaction",
            Update::Slot(_)        => "slot",
        }
    }

    pub fn as_account(&self) -> Option<&AccountUpdate> {
        match self {
            Update::Account(a) => Some(a),
            _                  => None,
        }
    }

    pub fn as_transaction(&self) -> Option<&TransactionUpdate> {
        match self {
            Update::Transaction(t) => Some(t),
            _                      => None,
        }
    }
}

// Helpers
pub(crate) fn parse_pubkey(bytes: &[u8]) -> Result<Pubkey, crate::error::StreamError> {
    if bytes.len() != 32 {
        return Err(crate::error::StreamError::decode(format!(
            "expected 32-byte pubkey, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(bytes);
    Ok(Pubkey::from(arr))
}
