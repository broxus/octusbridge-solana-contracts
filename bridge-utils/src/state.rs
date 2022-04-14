use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 3217)]
pub struct RelayRound {
    pub is_initialized: bool,
    pub round_number: u32,
    pub round_end: u32,
    pub relays: Vec<Pubkey>,
}

impl Sealed for RelayRound {}

impl IsInitialized for RelayRound {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
pub struct Proposal {
    pub is_initialized: bool,
    pub round_number: u32,
    pub required_votes: u32,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum Vote {
    None,
    Confirm,
    Reject,
}
