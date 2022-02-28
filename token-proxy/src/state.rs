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
    pub decimals: u8,
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

#[derive(Debug, BorshSerialize, BorshDeserialize, EnumAsInner)]
pub enum TokenKind {
    Ever {
        name: String,
    },
    Solana {
        mint: Pubkey,
        deposit_limit: u64,
        withdrawal_limit: u64,
    },
}
