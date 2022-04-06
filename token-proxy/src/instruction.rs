use crate::EverAddress;
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
        // Mint asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Withdrawal daily limit
        withdrawal_daily_limit: u64,
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
        // Vault asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Withdrawal daily limit
        withdrawal_daily_limit: u64,
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
        // Mint asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
        // Ever recipient address
        recipient: EverAddress,
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
        // Vault asset name
        name: String,
        // Unique transfer hash
        payload_id: Hash,
        // Ever recipient address
        recipient: EverAddress,
        // Deposit amount
        amount: u64,
    },

    /// Withdraw EVER/SOL request
    ///
    /// # Account references
    /// ...
    WithdrawRequest {
        // Mint asset name
        name: String,
        // Current round number
        round_number: u32,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Sender address
        sender: EverAddress,
        // Deposit amount
        amount: u64,
    },

    /// Confirm withdraw EVER/SOL request
    ///
    /// # Account references
    /// ...
    ConfirmWithdrawRequest {
        // Current round number
        round_number: u32,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Update withdraw status
    ///
    /// # Account references
    /// ...
    UpdateWithdrawStatus {
        // Mint asset name
        name: String,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Withdraw EVER
    ///
    /// # Account references
    /// ...
    WithdrawEver {
        /// Mint asset name
        name: String,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Withdraw SOL
    ///
    /// # Account references
    /// ...
    WithdrawSol {
        /// Mint asset name
        name: String,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Approve Withdraw Ever
    ///
    /// # Account references
    /// ...
    ApproveWithdrawEver {
        /// Mint asset name
        name: String,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Approve Withdraw SOL
    ///
    /// # Account references
    /// ...
    ApproveWithdrawSol {
        /// Mint asset name
        name: String,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Cancel Withdraw SOL
    ///
    /// # Account references
    /// ...
    CancelWithdrawSol {
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Unique transfer hash
        deposit_payload_id: Hash,
    },

    /// Force Withdraw SOL
    ///
    /// # Account references
    /// ...
    ForceWithdrawSol {
        // Mint asset name
        name: String,
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Fill Withdraw SOL
    ///
    /// # Account references
    /// ...
    /**/
    FillWithdrawSol {
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Unique transfer hash
        deposit_payload_id: Hash,
        // Recipient address
        recipient: EverAddress,
    },

    /// Transfer from Vault
    ///
    /// # Account references
    /// ...
    TransferFromVault {
        // Mint asset name
        name: String,
        // Amount to transfer
        amount: u64,
    },

    /// Change Bounty for Withdraw SOL
    ///
    /// # Account references
    /// ...
    ChangeBountyForWithdrawSol {
        // EVER->SOL event configuration
        event_configuration: String,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // New bounty value
        bounty: u64,
    },

    /// Change Settings
    ///
    /// # Account references
    /// ...
    ChangeSettings {
        // Token asset name
        name: String,
        // Emergency flag
        emergency: bool,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Withdrawal daily limit
        withdrawal_daily_limit: u64,
    },
}
