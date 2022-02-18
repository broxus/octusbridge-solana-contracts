use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use borsh::BorshSerialize;

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

use crate::utils;

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

#[derive(Debug)]
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
        let dst = array_mut_ref![dst, 0, RELAY_ROUND_PROPOSAL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            author,
            round_number,
            required_votes,
            is_executed,
            round_ttl,
            data_flat,
        ) = mut_array_refs![
            dst,
            1,
            32,
            4,
            4,
            1,
            4,
            4 + PUBKEY_BYTES * MAX_RELAYS + 4 + PUBKEY_BYTES * MAX_RELAYS
        ];

        utils::pack_bool(self.is_initialized, is_initialized);
        utils::pack_bool(self.is_executed, is_executed);

        author.copy_from_slice(self.author.as_ref());

        *round_number = self.round_number.to_le_bytes();
        *required_votes = self.required_votes.to_le_bytes();
        *round_ttl = self.round_ttl.to_le_bytes();

        let mut offset = 0;

        *array_mut_ref![data_flat, offset, 4] = (self.relays.len() as u32).to_le_bytes();
        offset += 4;

        for relay in &self.relays {
            let relays_flat = array_mut_ref![data_flat, offset, PUBKEY_BYTES];
            relays_flat.copy_from_slice(relay.as_ref());
            offset += PUBKEY_BYTES;
        }

        *array_mut_ref![data_flat, offset, 4] = (self.voters.len() as u32).to_le_bytes();
        offset += 4;

        for voter in &self.voters {
            let voters_flat = array_mut_ref![data_flat, offset, PUBKEY_BYTES];
            voters_flat.copy_from_slice(voter.as_ref());
            offset += PUBKEY_BYTES;
        }
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![src, 0, RELAY_ROUND_PROPOSAL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            author,
            round_number,
            required_votes,
            is_executed,
            round_ttl,
            data_flat,
        ) = array_refs![
            input,
            1,
            32,
            4,
            4,
            1,
            4,
            4 + PUBKEY_BYTES * MAX_RELAYS + 4 + PUBKEY_BYTES * MAX_RELAYS
        ];

        let is_initialized = utils::unpack_bool(is_initialized)?;
        let is_executed = utils::unpack_bool(is_executed)?;

        let author = Pubkey::new(author);

        let round_number = u32::from_le_bytes(*round_number);
        let required_votes = u32::from_le_bytes(*required_votes);
        let round_ttl = u32::from_le_bytes(*round_ttl);

        let mut offset = 0;

        let relays_len = u32::from_le_bytes(*array_ref![data_flat, offset, 4]);
        offset += 4;

        let mut relays = Vec::with_capacity(relays_len as usize);
        for _ in 0..relays_len {
            let relays_flat = array_ref![data_flat, offset, PUBKEY_BYTES];
            relays.push(Pubkey::new(relays_flat));
            offset += PUBKEY_BYTES;
        }

        let voters_len = u32::from_le_bytes(*array_ref![data_flat, offset, 4]);
        offset += 4;

        let mut voters = Vec::with_capacity(voters_len as usize);
        for _ in 0..voters_len {
            let voters_flat = array_ref![data_flat, offset, PUBKEY_BYTES];
            voters.push(Pubkey::new(voters_flat));
            offset += PUBKEY_BYTES;
        }

        Ok(Self {
            is_initialized,
            author,
            round_number,
            required_votes,
            is_executed,
            round_ttl,
            relays,
            voters,
        })
    }
}

#[derive(Debug, BorshSerialize)]
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
        let dst = array_mut_ref![dst, 0, RELAY_ROUND_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, round_number, round_ttl, relays_len, relays_flat) =
            mut_array_refs![dst, 1, 4, 4, 4, PUBKEY_BYTES * MAX_RELAYS];

        utils::pack_bool(self.is_initialized, is_initialized);

        *round_number = self.round_number.to_le_bytes();
        *round_ttl = self.round_ttl.to_le_bytes();

        *relays_len = (self.relays.len() as u32).to_le_bytes();

        let mut offset = 0;

        for relay in &self.relays {
            let relay_flat = array_mut_ref![relays_flat, offset, PUBKEY_BYTES];
            relay_flat.copy_from_slice(relay.as_ref());
            offset += PUBKEY_BYTES;
        }
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![src, 0, RELAY_ROUND_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, round_number, round_ttl, relays_len, relays_flat) =
            array_refs![input, 1, 4, 4, 4, PUBKEY_BYTES * MAX_RELAYS];

        let is_initialized = utils::unpack_bool(is_initialized)?;

        let round_number = u32::from_le_bytes(*round_number);
        let round_ttl = u32::from_le_bytes(*round_ttl);

        let relays_len = u32::from_le_bytes(*relays_len);

        let mut relays = Vec::with_capacity(relays_len as usize);

        let mut offset = 0;
        for _ in 0..relays_len {
            let relay_flat = array_ref![relays_flat, offset, PUBKEY_BYTES];
            relays.push(Pubkey::new(relay_flat));
            offset += PUBKEY_BYTES;
        }

        Ok(Self {
            is_initialized,
            round_number,
            round_ttl,
            relays,
        })
    }
}

#[derive(Debug, BorshSerialize)]
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
        let dst = array_mut_ref![dst, 0, SETTINGS_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, round_number) = mut_array_refs![dst, 1, 4];

        utils::pack_bool(self.is_initialized, is_initialized);

        *round_number = self.round_number.to_le_bytes();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![src, 0, SETTINGS_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, round_number) = array_refs![input, 1, 4];

        let is_initialized = utils::unpack_bool(is_initialized)?;

        let round_number = u32::from_le_bytes(*round_number);

        Ok(Self {
            is_initialized,
            round_number,
        })
    }
}
