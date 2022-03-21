use borsh::{BorshDeserialize, BorshSerialize};
use enum_as_inner::EnumAsInner;

use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

use bridge_derive::BridgePack;

pub const WITHDRAWAL_PERIOD: i64 = 86400;

const WITHDRAW_EVENT_LEN: usize = 1  // decimals
    + PUBKEY_BYTES                   // solana recipient address
    + PUBKEY_BYTES + 1               // ever sender address
    + 8                              // amount
;

const WITHDRAW_EVER_META_LEN: usize = PUBKEY_BYTES  // author
    + 1 + PUBKEY_BYTES                              // kind
    + 8                                             // bounty
    + 1                                             // status
;

const WITHDRAW_SOL_META_LEN: usize = PUBKEY_BYTES   // author
    + 1 + PUBKEY_BYTES + PUBKEY_BYTES               // kind
    + 8                                             // bounty
    + 1                                             // status
;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 500)]
pub struct Settings {
    pub is_initialized: bool,
    pub kind: TokenKind,
    pub admin: Pubkey,
    pub decimals: u8,
    pub emergency: bool,
    pub deposit_limit: u64,
    pub withdrawal_limit: u64,
    pub withdrawal_daily_limit: u64,
    pub withdrawal_daily_amount: u64,
    pub withdrawal_ttl: i64,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 500)]
pub struct Deposit {
    pub is_initialized: bool,
    pub payload_id: Hash,
    pub kind: TokenKind,
    pub sender: Pubkey,
    pub recipient: EverAddress,
    pub decimals: u8,
    pub amount: u64,
}

impl Sealed for Deposit {}

impl IsInitialized for Deposit {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
pub struct Withdrawal {
    pub is_initialized: bool,
    pub payload_id: Hash,
    pub round_number: u32,
    pub required_votes: u32,
    pub signers: Vec<Pubkey>,
    pub event: WithdrawalEvent,
    pub meta: WithdrawalMeta,
}

impl Sealed for Withdrawal {}

impl IsInitialized for Withdrawal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
pub struct WithdrawalPattern {
    pub is_initialized: bool,
    pub payload_id: Hash,
    pub round_number: u32,
    pub required_votes: u32,
    pub signers: Vec<Pubkey>,
    pub event: Vec<u8>,
    pub meta: Vec<u8>,
}

impl Sealed for WithdrawalPattern {}

impl IsInitialized for WithdrawalPattern {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct WithdrawalEvent {
    pub event_len: u32,
    pub decimals: u8,
    pub recipient: Pubkey,
    pub sender: EverAddress,
    pub amount: u64,
}

impl WithdrawalEvent {
    pub fn new(decimals: u8, recipient: Pubkey, sender: EverAddress, amount: u64) -> Self {
        Self {
            event_len: WITHDRAW_EVENT_LEN as u32,
            decimals,
            recipient,
            sender,
            amount,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct WithdrawalMeta {
    pub meta_len: u32,
    pub author: Pubkey,
    pub kind: TokenKind,
    pub status: WithdrawalStatus,
    pub bounty: u64,
}

impl WithdrawalMeta {
    pub fn new(author: Pubkey, kind: TokenKind, status: WithdrawalStatus, bounty: u64) -> Self {
        let meta_len = match kind {
            TokenKind::Ever { .. } => WITHDRAW_EVER_META_LEN,
            TokenKind::Solana { .. } => WITHDRAW_SOL_META_LEN,
        } as u32;

        Self {
            meta_len,
            author,
            kind,
            status,
            bounty,
        }
    }
}

#[derive(Debug, Copy, Clone, BorshSerialize, BorshDeserialize, EnumAsInner, PartialEq, Eq)]
pub enum TokenKind {
    Ever { mint: Pubkey },
    Solana { mint: Pubkey, vault: Pubkey },
}

#[derive(Copy, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum WithdrawalStatus {
    New,
    Processed,
    Cancelled,
    Pending,
    WaitingForApprove,
}

#[derive(Debug, Clone, Copy, Default, BorshSerialize, BorshDeserialize)]
pub struct EverAddress {
    pub workchain_id: i8,
    pub address: Pubkey,
}
