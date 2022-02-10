use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum RoundLoaderInstruction {
    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    ///   2. []                 Rent sysvar
    ///   3. []                 System program
    CreateProposal {
        /// Relay Round number
        round: u32,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    WriteProposal {
        /// Relay Round number
        round: u32,

        /// Offset at which to write the given bytes
        offset: u32,

        /// Serialized Relay Round data
        bytes: Vec<u8>,
    },

    /// Finalize an proposal account loaded with a new Relay Round data
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    The account to prepare for execution
    ///   1. [WRITE]            Proposal account
    FinalizeProposal {
        /// Relay Round number
        round: u32,
    },

    /// Vote for proposal
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    Vote,
}
