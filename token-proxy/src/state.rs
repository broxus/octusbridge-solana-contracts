use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 115)]
pub struct Settings {
    pub is_initialized: bool,
    pub name: String,
    pub kind: TokenKind,
    pub withdrawal_limit: u64,
    pub deposit_limit: u64,
    pub decimals: u8,
    pub admin: Pubkey,
    pub token: Pubkey,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Copy, BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum TokenKind {
    Ever = 0,
    Solana = 1,
}
