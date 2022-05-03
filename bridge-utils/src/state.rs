use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::types::Vote;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Proposal {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
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

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct PDA {
    pub author: Pubkey,
    pub settings: Pubkey,
    pub event_timestamp: u32,
    pub event_transaction_lt: u64,
    pub event_configuration: Pubkey,
}

#[derive(
    Debug, Copy, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq,
)]
pub enum AccountKind {
    Deposit,
    Proposal,
    Settings,
    RelayRound,
}
