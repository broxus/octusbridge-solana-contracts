use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;
use bridge_utils::EverAddress;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

pub const WITHDRAWAL_TOKEN_PERIOD: i64 = 86400;

const WITHDRAW_TOKEN_EVENT_LEN: usize = 1   // decimals
    + PUBKEY_BYTES                          // solana recipient address
    + PUBKEY_BYTES + 1 + 1                  // ever sender address
    + 8                                     // amount
;

const WITHDRAW_TOKEN_META_LEN: usize = PUBKEY_BYTES // author
    + 8                                             // bounty
    + 1                                             // status
;

const DEPOSIT_TOKEN_EVENT_LEN: usize = 1    // decimals
    + PUBKEY_BYTES + 1 + 1                  // ever recipient address
    + PUBKEY_BYTES                          // solana sender address
    + 8                                     // amount
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
    pub kind: TokenKind,
    pub event: Vec<u8>,
}

impl Sealed for Deposit {}

impl IsInitialized for Deposit {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 500)]
pub struct DepositToken {
    pub is_initialized: bool,
    pub kind: TokenKind,
    pub event: DepositTokenEventWithLen,
}

impl Sealed for DepositToken {}

impl IsInitialized for DepositToken {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositTokenEvent {
    pub decimals: u8,
    pub recipient: EverAddress,
    pub sender: Pubkey,
    pub amount: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositTokenEventWithLen {
    pub len: u32,
    pub data: DepositTokenEvent,
}

impl DepositTokenEventWithLen {
    pub fn new(decimals: u8, recipient: EverAddress, sender: Pubkey, amount: u64) -> Self {
        Self {
            len: DEPOSIT_TOKEN_EVENT_LEN as u32,
            data: DepositTokenEvent {
                decimals,
                recipient,
                sender,
                amount,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
pub struct Withdrawal {
    pub is_initialized: bool,
    pub kind: TokenKind,
    pub round_number: u32,
    pub required_votes: u32,
    pub signers: Vec<Vote>,
    pub event: Vec<u8>,
    pub meta: Vec<u8>,
}

impl Sealed for Withdrawal {}

impl IsInitialized for Withdrawal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
pub struct WithdrawalToken {
    pub is_initialized: bool,
    pub kind: TokenKind,
    pub round_number: u32,
    pub required_votes: u32,
    pub signers: Vec<Vote>,
    pub event: WithdrawalTokenEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
}

impl Sealed for WithdrawalToken {}

impl IsInitialized for WithdrawalToken {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalTokenEvent {
    pub decimals: u8,
    pub recipient: Pubkey,
    pub sender: EverAddress,
    pub amount: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalTokenEventWithLen {
    pub len: u32,
    pub data: WithdrawalTokenEvent,
}

impl WithdrawalTokenEventWithLen {
    pub fn new(decimals: u8, recipient: Pubkey, sender: EverAddress, amount: u64) -> Self {
        Self {
            len: WITHDRAW_TOKEN_EVENT_LEN as u32,
            data: WithdrawalTokenEvent {
                decimals,
                recipient,
                sender,
                amount,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalTokenMeta {
    pub author: Pubkey,
    pub status: WithdrawalTokenStatus,
    pub bounty: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalTokenMetaWithLen {
    pub len: u32,
    pub data: WithdrawalTokenMeta,
}

impl WithdrawalTokenMetaWithLen {
    pub fn new(author: Pubkey, status: WithdrawalTokenStatus, bounty: u64) -> Self {
        Self {
            len: WITHDRAW_TOKEN_META_LEN as u32,
            data: WithdrawalTokenMeta {
                author,
                status,
                bounty,
            },
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    EnumAsInner,
    PartialEq,
    Eq,
)]
pub enum TokenKind {
    Ever { mint: Pubkey },
    Solana { mint: Pubkey, vault: Pubkey },
}

#[derive(
    Copy, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq,
)]
pub enum WithdrawalTokenStatus {
    New,
    Processed,
    Cancelled,
    Pending,
    WaitingForApprove,
    WaitingForRelease,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum Vote {
    None,
    Confirm,
    Reject,
}
