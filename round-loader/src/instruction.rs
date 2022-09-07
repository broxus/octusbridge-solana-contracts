use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::Vote;
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum RoundLoaderInstruction {
    /// Vote for proposal
    ///
    /// # Account references
    /// ...
    VoteForProposal {
        // Vote type
        vote: Vote,
    },

    /// Initialize the genesis round
    ///
    /// # Account references
    /// ...
    Initialize {
        // Genesis round number
        genesis_round_number: u32,
        // Relay Round submitter role
        round_submitter: Pubkey,
        // Minimum required votes
        min_required_votes: u32,
        // Round TTL
        round_ttl: u32,
    },

    /// Update Settings
    ///
    /// # Account references
    /// ...
    UpdateSettings {
        // Current round number
        current_round_number: Option<u32>,
        // Relay Round submitter role
        round_submitter: Option<Pubkey>,
        // Minimum required votes
        min_required_votes: Option<u32>,
        // Round TTL
        round_ttl: Option<u32>,
    },

    /// Create Relay Round
    ///
    /// # Account references
    /// ...
    CreateRelayRound {
        // Relay Round number
        round_number: u32,
        // Relays keys in a new round
        relays: Vec<Pubkey>,
        // Round end
        round_end: u32,
    },

    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    /// ...
    CreateProposal {
        // Round number
        round_number: u32,
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
        // Sha256 of event data
        event_data: Hash,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    /// ...
    WriteProposal {
        // Offset at which to write the given bytes
        offset: u32,
        // Serialized set of keys of for a new round
        bytes: Vec<u8>,
    },

    /// Finalize an proposal account loaded with a new Relay Round data
    ///
    /// # Account references
    /// ...
    FinalizeProposal,

    /// Execute proposal
    ///
    /// # Account references
    /// ...
    ExecuteProposal,
}
