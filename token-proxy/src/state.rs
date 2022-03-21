use borsh::{BorshDeserialize, BorshSerialize};
use enum_as_inner::EnumAsInner;

use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

use bridge_derive::BridgePack;

pub const WITHDRAWAL_PERIOD: i64 = 86400;

const WITHDRAW_EVENT_LEN: usize = 1  // solana receiver address
    + PUBKEY_BYTES                   // solana decimals
    + PUBKEY_BYTES + 1               // ever sender address
    + 8                              // amount
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
    pub event: WithdrawalEvent,
    pub meta: WithdrawalMeta,
    pub required_votes: u32,
    pub signers: Vec<Pubkey>,
}

impl Sealed for Withdrawal {}

impl IsInitialized for Withdrawal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Sealed for WithdrawalEvent {}

impl IsInitialized for WithdrawalEvent {
    fn is_initialized(&self) -> bool {
        self.is_initialized
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

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct WithdrawalEvent {
    pub is_initialized: bool,
    pub event_len: u32,
    pub decimals: u8,
    pub recipient: Pubkey,
    pub sender: EverAddress,
    pub amount: u64,
}

impl WithdrawalEvent {
    pub fn new(decimals: u8, recipient: Pubkey, sender: EverAddress, amount: u64) -> Self {
        Self {
            is_initialized: true,
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
    pub author: Pubkey,
    pub kind: TokenKind,
    pub status: WithdrawalStatus,
    pub bounty: u64,
}

#[derive(Debug, Clone, Copy, Default, BorshSerialize, BorshDeserialize)]
pub struct EverAddress {
    pub workchain_id: i8,
    pub address: Pubkey,
}
