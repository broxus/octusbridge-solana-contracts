use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;
use bridge_utils::state::{AccountKind, PDA};
use bridge_utils::types::Vote;

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

/// Minimum Relays in round
pub const MIN_RELAYS: usize = 3;

/// Maximum Relays in round
pub const MAX_RELAYS: usize = 100;

pub const LOAD_DATA_BEGIN_OFFSET: usize = 1 // is_executed
    + 1                                     // account_kind
    + 4                                     // round_number
    + 4                                     // required_votes
    + PUBKEY_BYTES                          // author
    + PUBKEY_BYTES                          // settings
    + 4                                     // event_timestamp
    + 8                                     // event_transaction_lt
    + PUBKEY_BYTES                          // event_configuration
;

pub const LOAD_DATA_END_OFFSET: usize = LOAD_DATA_BEGIN_OFFSET
    + 4                             // relays len
    + 4                             // round_num
    + PUBKEY_BYTES * MAX_RELAYS     // relays
    + 4                             // round_end
;

const RELAY_ROUND_PROPOSAL_META_LEN: usize = 1  // is_executed
;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 6)]
pub struct Settings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub round_number: u32,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 5000)]
pub struct RelayRound {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
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
pub struct RelayRoundProposal {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: RelayRoundProposalEventWithLen,
    pub meta: RelayRoundProposalMetaWithLen,
    pub signers: Vec<Vote>,
}

impl Sealed for RelayRoundProposal {}

impl IsInitialized for RelayRoundProposal {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RelayRoundProposalEvent {
    pub round_num: u32,
    pub relays: Vec<Pubkey>,
    pub round_end: u32,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RelayRoundProposalEventWithLen {
    pub len: u32,
    pub data: RelayRoundProposalEvent,
}

impl RelayRoundProposalEventWithLen {
    pub fn new(round_num: u32, relays: Vec<Pubkey>, round_end: u32) -> Result<Self, ProgramError> {
        Ok(Self {
            len: 4 + relays.try_to_vec()?.len() as u32 + 4,
            data: RelayRoundProposalEvent {
                round_num,
                relays,
                round_end,
            },
        })
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RelayRoundProposalMeta {
    pub is_executed: bool,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RelayRoundProposalMetaWithLen {
    pub len: u32,
    pub data: RelayRoundProposalMeta,
}

impl RelayRoundProposalMetaWithLen {
    pub fn new() -> Self {
        Self {
            len: RELAY_ROUND_PROPOSAL_META_LEN as u32,
            data: RelayRoundProposalMeta { is_executed: false },
        }
    }
}

impl Default for RelayRoundProposalMetaWithLen {
    fn default() -> Self {
        Self::new()
    }
}
