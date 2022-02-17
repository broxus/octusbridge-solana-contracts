use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum RoundLoaderInstruction {
    /// Initialize the first round
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Authority account of Round Loader program
    ///   1. [WRITE]            Settings account
    ///   2. [WRITE]            The first Relay Round account
    ///   3. []                 Buffer Program account
    ///   4. []                 System program
    Initialize {
        /// Genesis Relay Round number
        round: u32,
        /// TTL of round
        round_ttl: u32,
    },

    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    ///   2. []                 Settings account
    ///   3. []                 Current Round account
    ///   4. []                 System program
    CreateProposal {
        /// New Relay Round number
        round: u32,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    WriteProposal {
        /// New Relay Round number
        round: u32,

        /// Offset at which to write the given bytes
        offset: u32,

        /// Serialized Relay Round data
        bytes: Vec<u8>,
    },

    /// Finalize an proposal account loaded with a new Relay Round data
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    ///   1. []                 Settings account
    ///   2. []                 Current Round account
    FinalizeProposal {
        /// New Relay Round number
        round: u32,
    },

    /// Vote for proposal
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Relay account
    ///   1. [WRITE]            Proposal account
    ///   2. [WRITE]            Settings account
    ///   3. [WRITE]            New Round account
    ///   4. []                 Current Round account
    ///   5. []                 System program
    Vote,
}
