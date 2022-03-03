use borsh::{BorshDeserialize, BorshSerialize};
use enum_as_inner::EnumAsInner;

use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

use bridge_derive::BridgePack;

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
    pub recipient: Pubkey,
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
    pub kind: TokenKind,
    pub author: Pubkey,
    pub recipient: Pubkey,
    pub required_votes: u32,
    pub signers: Vec<Pubkey>,
    pub status: WithdrawalStatus,
    pub amount: u64,
    pub bounty: u64,
}

impl Sealed for Withdrawal {}

impl IsInitialized for Withdrawal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, EnumAsInner, PartialEq, Eq)]
pub enum TokenKind {
    Ever { mint: Pubkey },
    Solana { mint: Pubkey, vault: Pubkey },
}

#[derive(Copy, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum WithdrawalStatus {
    New,
    Expired,
    Processed,
    Cancelled,
    Pending,
    WaitingForApprove,
}
