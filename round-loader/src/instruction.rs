use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum RoundLoaderInstruction {
    /// Initialize the first round
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Creator account
    ///   2. [WRITE]            Settings account
    ///   3. [WRITE]            The first Relay Round account
    ///   4. []                 Buffer Program account
    ///   5. []                 System program
    ///   6. []                 The rent sysvar
    Initialize {
        /// Genesis Relay Round number
        round: u32,
        /// TTL of round
        round_ttl: u32,
    },

    /// Create proposal account for a new Relay Round
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Creator account
    ///   2. [WRITE]            Proposal account
    ///   3. []                 Settings account
    ///   4. []                 Current Round account
    ///   5. []                 System program
    ///   6. []                 The rent sysvar
    CreateProposal {
        /// New Relay Round number
        round: u32,
    },

    /// Write Relay Round data into an proposal account
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Creator account
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
    ///   0. [WRITE, SIGNER]    Creator account
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
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Voter account
    ///   2. [WRITE]            Proposal account
    ///   3. [WRITE]            Settings account
    ///   4. [WRITE]            New Round account
    ///   5. []                 Current Round account
    ///   6. []                 System program
    ///   7. []                 The rent sysvar
    Vote,
}
