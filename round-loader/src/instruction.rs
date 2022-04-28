use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::Vote;
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

    /// Initialize the first round
    ///
    /// # Account references
    /// ...
    Initialize {
        // Genesis Relay Round number
        round_number: u32,
        // End of round
        round_end: u32,
        // Relays keys
        relays: Vec<Pubkey>,
    },

    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    /// ...
    CreateProposal {
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    /// ...
    WriteProposal {
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
        // Offset at which to write the given bytes
        offset: u32,
        // Serialized set of keys of for a new round
        bytes: Vec<u8>,
    },

    /// Finalize an proposal account loaded with a new Relay Round data
    ///
    /// # Account references
    /// ...
    FinalizeProposal {
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
    },

    /// Execute proposal
    ///
    /// # Account references
    /// ...
    ExecuteProposal,
}
