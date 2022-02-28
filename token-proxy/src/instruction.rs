use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TokenProxyInstruction {
    /// Initialize Mint Account
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Initializer account
    ///   ..
    InitializeMint {
        /// Mint asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
    },

    /// Initialize Vault Account
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Initializer account
    ///   ..
    InitializeVault {
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
    },

    /// Deposit SOL
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Initializer account
    ///   ..
    DepositSolana {
        // Unique transfer hash
        payload_id: Hash,
        // Ever recipient address
        recipient: Pubkey,
        // Deposit amount
        amount: u64,
    },
}
