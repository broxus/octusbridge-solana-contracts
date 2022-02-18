use borsh::{BorshDeserialize, BorshSerialize};

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
    + 4                          // round_ttl
    + 4                          // relays len
    + PUBKEY_BYTES * MAX_RELAYS; // relays

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RelayRoundProposal {
    pub is_initialized: bool,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub is_executed: bool,
    pub round_ttl: u32,
    pub relays: Vec<Pubkey>,
    pub voters: Vec<Pubkey>,
}

impl Sealed for RelayRoundProposal {}

impl IsInitialized for RelayRoundProposal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

const RELAY_ROUND_PROPOSAL_LEN: usize = 1 // is_initialized
    + PUBKEY_BYTES               // author
    + 4                          // round_number
    + 4                          // required_votes
    + 1                          // is_executed
    + 4                          // round_ttl
    + 4                          // relays len
    + PUBKEY_BYTES * MAX_RELAYS  // relays
    + 4                          // voters len
    + PUBKEY_BYTES * MAX_RELAYS; // voters

impl Pack for RelayRoundProposal {
    const LEN: usize = RELAY_ROUND_PROPOSAL_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut data = self.try_to_vec().unwrap();
        let (left, _) = dst.split_at_mut(data.len());
        left.copy_from_slice(&mut data);
    }

    fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::deserialize(&mut src)?;
        Ok(unpacked)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RelayRound {
    pub is_initialized: bool,
    pub round_number: u32,
    pub round_ttl: u32,
    pub relays: Vec<Pubkey>,
}

impl Sealed for RelayRound {}

impl IsInitialized for RelayRound {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

const RELAY_ROUND_LEN: usize = 1 // is_initialized
    + 4                          // round_number
    + 4                          // round_ttl
    + 4                          // relays len
    + PUBKEY_BYTES * MAX_RELAYS; // relays

impl Pack for RelayRound {
    const LEN: usize = RELAY_ROUND_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut data = self.try_to_vec().unwrap();
        let (left, _) = dst.split_at_mut(data.len());
        left.copy_from_slice(&mut data);
    }

    fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::deserialize(&mut src)?;
        Ok(unpacked)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
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

const SETTINGS_LEN: usize = 1 // is_initialized
    + 4; // round_number

impl Pack for Settings {
    const LEN: usize = SETTINGS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut data = self.try_to_vec().unwrap();
        let (left, _) = dst.split_at_mut(data.len());
        left.copy_from_slice(&mut data);
    }

    fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::deserialize(&mut src)?;
        Ok(unpacked)
    }
}
