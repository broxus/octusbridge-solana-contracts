use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

/// Maximum Relays in round
pub const MAX_RELAYS: usize = 100;

pub const LOAD_DATA_BEGIN_OFFSET: usize = 1
    + PUBKEY_BYTES // author
    + 4  // round_number
    + 4  // required_votes
    + 1; // is_executed

pub const LOAD_DATA_END_OFFSET: usize = LOAD_DATA_BEGIN_OFFSET
    + 8                          // round_ttl
    + 4                          // relays len
    + PUBKEY_BYTES * MAX_RELAYS; // relays

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 6458)]
pub struct RelayRoundProposal {
    pub is_initialized: bool,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub is_executed: bool,
    pub round_ttl: i64,
    pub relays: Vec<Pubkey>,
    pub voters: Vec<Pubkey>,
}

impl Sealed for RelayRoundProposal {}

impl IsInitialized for RelayRoundProposal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 3217)]
pub struct RelayRound {
    pub is_initialized: bool,
    pub round_number: u32,
    pub round_ttl: i64,
    pub relays: Vec<Pubkey>,
}

impl Sealed for RelayRound {}

impl IsInitialized for RelayRound {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5)]
pub struct Settings {
    pub is_initialized: bool,
    pub round_number: u32,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
