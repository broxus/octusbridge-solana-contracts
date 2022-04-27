use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;
use serde::{Deserialize, Serialize};

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

use super::types::Vote;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
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

impl Sealed for Proposal {}

impl IsInitialized for Proposal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct PDA {
    pub author: Pubkey,
    pub settings: Pubkey,
    pub event_timestamp: u32,
    pub event_transaction_lt: u64,
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
