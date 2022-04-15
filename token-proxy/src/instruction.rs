use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::{EverAddress, UInt256, Vote};

use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TokenProxyInstruction {
    /// Initialize Mint Account
    ///
    /// # Account references
    /// ...
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
    /// ...
    InitializeVault {
        // Vault asset name
        name: String,
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
    /// ...
    DepositEver {
        // Mint asset name
        name: String,
        // Ever recipient address
        recipient: EverAddress,
        // Deposit amount
        amount: u64,
        // Deposit seed
        deposit_seed: u64,
    },

    /// Deposit SOL
    ///
    /// # Account references
    /// ...
    DepositSol {
        // Vault asset name
        name: String,
        // Ever recipient address
        recipient: EverAddress,
        // Deposit amount
        amount: u64,
        // Deposit seed
        deposit_seed: u64,
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
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Sender address
        sender_address: EverAddress,
        // Sender address
        recipient_address: Pubkey,
        // Withdrawal amount
        amount: u64,
    },

    /// Vote for withdraw EVER/SOL request
    ///
    /// # Account references
    /// ...
    VoteForWithdrawRequest {
        // Current round number
        round_number: u32,
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Vote type
        vote: Vote,
    },

    /// Update withdraw status
    ///
    /// # Account references
    /// ...
    UpdateWithdrawStatus {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Withdraw EVER
    ///
    /// # Account references
    /// ...
    WithdrawEver {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Withdraw SOL
    ///
    /// # Account references
    /// ...
    WithdrawSol {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Approve Withdraw Ever
    ///
    /// # Account references
    /// ...
    ApproveWithdrawEver {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Approve Withdraw SOL
    ///
    /// # Account references
    /// ...
    ApproveWithdrawSol {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
    },

    /// Cancel Withdraw SOL
    ///
    /// # Account references
    /// ...
    CancelWithdrawSol {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Deposit seed
        deposit_seed: u64,
    },
    /// Force Withdraw SOL
    ///
    /// # Account references
    /// ...
    ForceWithdrawSol {
        // EVER->SOL event configuration
        event_configuration: UInt256,
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
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Recipient address
        recipient: EverAddress,
        // Deposit seed
        deposit_seed: u64,
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
        event_configuration: UInt256,
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
