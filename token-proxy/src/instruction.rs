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
    ///   2. [WRITE]            Mint account
    ///   3. [WRITE]            Settings account
    ///   4. []                 Buffer Program account
    ///   5. []                 System program
    ///   6. []                 Token program
    ///   7. []                 The rent sysvar
    InitializeMint {
        /// Mint asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Admin pubkey
        admin: Pubkey,
    },

    /// Initialize Vault Account
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Initializer account
    ///   2. [WRITE]            Vault account
    ///   3. [WRITE]            Mint account
    ///   4. [WRITE]            Settings account
    ///   5. []                 Buffer Program account
    ///   6. []                 System program
    ///   7. []                 Token program
    ///   8. []                 The rent sysvar
    InitializeVault {
        /// Vault asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Admin pubkey
        admin: Pubkey,
    },

    /// Deposit EVER
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Sender account
    ///   2. [WRITE]            Sender token account
    ///   3. [WRITE]            Deposit account
    ///   4. [WRITE]            Mint account
    ///   5. []                 Settings Program account
    ///   6. []                 System program
    ///   7. []                 Token program
    ///   8. []                 The rent sysvar
    DepositEver {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
        // Ever recipient address
        recipient: Pubkey,
        // Deposit amount
        amount: u64,
    },

    /// Deposit SOL
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Sender account
    ///   2. [WRITE]            Sender token account
    ///   3. [WRITE]            Vault account
    ///   4. [WRITE]            Deposit account
    ///   5. []                 Mint account
    ///   6. []                 Settings Program account
    ///   7. []                 System program
    ///   8. []                 Token program
    ///   9. []                 The rent sysvar
    DepositSol {
        /// Vault asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
        // Ever recipient address
        recipient: Pubkey,
        // Deposit amount
        amount: u64,
    },

    /// Withdraw EVER request
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    WithdrawRequest {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
        // Current round number
        round_number: u32,
        // Deposit amount
        amount: u64,
    },

    /// Confirm withdraw EVER request
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    ConfirmWithdrawRequest {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
        // Current round number
        round_number: u32,
    },

    /// Withdraw EVER
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    WithdrawEver {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
    },

    /// Withdraw SOL
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    WithdrawSol {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
    },

    /// Approve Withdraw Ever
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    ApproveWithdrawEver {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
    },

    /// Approve Withdraw SOL
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    ApproveWithdrawSol {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
    },

    /// Force Withdraw SOL
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    /// ...
    ForceWithdrawSol {
        /// Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
    },
}
