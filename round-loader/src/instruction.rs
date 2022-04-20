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
        // Proposal seed
        proposal_seed: u128,
        // Settings address
        settings_address: Pubkey,
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
    },

    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    /// ...
    CreateProposal {
        // Proposal seed
        proposal_seed: u128,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    /// ...
    WriteProposal {
        // Proposal seed
        proposal_seed: u128,
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
        // Proposal seed
        proposal_seed: u128,
    },

    /// Execute proposal
    ///
    /// # Account references
    /// ...
    ExecuteProposal,
}
