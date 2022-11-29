use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;
use bridge_utils::state::{AccountKind, PDA};
use bridge_utils::types::Vote;
use serde::{Deserialize, Serialize};

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

/// Minimum Relays in round
pub const MIN_RELAYS: usize = 3;

/// Maximum Relays in round
pub const MAX_RELAYS: usize = 100;

pub const LOAD_DATA_BEGIN_OFFSET: usize = 1 // is_executed
    + 2                                     // account_kind
    + 1                                     // is_executed
    + PUBKEY_BYTES                          // author
    + 4                                     // round_number
    + 4                                     // required_votes
    + 4                                     // event_timestamp
    + 8                                     // event_transaction_lt
    + PUBKEY_BYTES                          // event_configuration
;

pub const LOAD_DATA_END_OFFSET: usize = LOAD_DATA_BEGIN_OFFSET
    + 4                                         // relays len
    + 4                                         // round_num
    + 4 + PUBKEY_BYTES * MAX_RELAYS             // relays
    + 4                                         // round_end
;

const RELAY_ROUND_PROPOSAL_META_LEN: usize = 0  // empty
;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 100)] // 46 + reserve
pub struct Settings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub current_round_number: u32,
    pub round_submitter: Pubkey,
    pub min_required_votes: u32,
    pub round_ttl: u32,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 3215)]
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
#[bridge_pack(length = 3414)]
pub struct RelayRoundProposal {
    pub is_initialized: bool,
    pub is_executed: bool,
    pub account_kind: AccountKind,
    pub author: Pubkey,
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

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct RelayRoundProposalEvent {
    pub round_num: u32,
    pub relays: Vec<Pubkey>,
    pub round_end: u32,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct RelayRoundProposalEventWithLen {
    pub len: u32,
    pub data: RelayRoundProposalEvent,
}

impl RelayRoundProposalEventWithLen {
    pub fn new(round_num: u32, relays: Vec<Pubkey>, round_end: u32) -> Self {
        Self {
            len: (4 + 4 + 4 + PUBKEY_BYTES * relays.len()) as u32,
            data: RelayRoundProposalEvent {
                round_num,
                round_end,
                relays,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct RelayRoundProposalMeta {}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct RelayRoundProposalMetaWithLen {
    pub len: u32,
    pub data: RelayRoundProposalMeta,
}

impl RelayRoundProposalMetaWithLen {
    pub fn new() -> Self {
        Self {
            len: RELAY_ROUND_PROPOSAL_META_LEN as u32,
            data: RelayRoundProposalMeta {},
        }
    }
}

impl Default for RelayRoundProposalMetaWithLen {
    fn default() -> Self {
        Self::new()
    }
}
