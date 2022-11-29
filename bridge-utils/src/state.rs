use borsh::{BorshDeserialize, BorshSerialize};
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::types::Vote;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Proposal {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub is_executed: bool,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: Vec<u8>,
    pub meta: Vec<u8>,
    pub signers: Vec<Vote>,
}

impl Proposal {
    pub fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        let (left, _) = dst.split_at_mut(data.len());
        left.copy_from_slice(&data);
    }

    pub fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::deserialize(&mut src)?;
        Ok(unpacked)
    }
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct PDA {
    pub event_timestamp: u32,
    pub event_transaction_lt: u64,
    pub event_configuration: Pubkey,
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
pub enum AccountKind {
    Settings(u8),
    Deposit(u8),
    Proposal(u8),
    RelayRound(u8),
    MultiVault(u8),
    TokenSettings(u8, u8),
}

impl AccountKind {
    pub fn to_value(&self) -> u8 {
        match self {
            AccountKind::Settings(_) => 0,
            AccountKind::Deposit(_) => 1,
            AccountKind::Proposal(_) => 2,
            AccountKind::RelayRound(_) => 3,
            AccountKind::MultiVault(_) => 4,
            AccountKind::TokenSettings(_, _) => 5,
        }
    }
}
