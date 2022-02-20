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

pub fn pack_token_kind(kind: TokenKind, dst: &mut [u8; 1]) {
    *dst = (kind as u8).to_le_bytes()
}

pub fn unpack_token_kind(src: &[u8; 1]) -> Result<TokenKind, ProgramError> {
    match u8::from_le_bytes(*src) {
        0 => Ok(TokenKind::Ever),
        1 => Ok(TokenKind::Solana),
        _ => Err(ProgramError::InvalidAccountData),
    }
}
