use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::{UInt256, Vote};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum RoundLoaderInstruction {
    /// Initialize the first round
    ///
    /// # Account references
    /// ...
    Initialize {
        // Genesis Relay Round number
        round_number: u32,
        // End of round
        round_end: u32,
    },

    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    /// ...
    CreateProposal {
        // Event configuration address
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    /// ...
    WriteProposal {
        // Event configuration address
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
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
        // Event configuration address
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Vote for proposal
    ///
    /// # Account references
    /// ...
    VoteForProposal {
        // Event configuration address
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Vote type
        vote: Vote,
    },

    /// Execute proposal
    ///
    /// # Account references
    /// ...
    ExecuteProposal,
}
