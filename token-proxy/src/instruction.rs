use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::{EverAddress, Vote};

use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TokenProxyInstruction {
    /// Vote for withdraw EVER/SOL request
    ///
    /// # Account references
    /// ...
    VoteForWithdrawRequest {
        // Withdrawal seed
        withdrawal_seed: u64,
        // Settings address
        settings_address: Pubkey,
        // Vote type
        vote: Vote,
    },

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
        // Deposit seed
        deposit_seed: u64,
        // Ever recipient address
        recipient_address: EverAddress,
        // Deposit amount
        amount: u64,
    },

    /// Deposit SOL
    ///
    /// # Account references
    /// ...
    DepositSol {
        // Deposit seed
        deposit_seed: u64,
        // Ever recipient address
        recipient_address: EverAddress,
        // Deposit amount
        amount: u64,
    },

    /// Withdraw EVER/SOL request
    ///
    /// # Account references
    /// ...
    WithdrawRequest {
        // Withdrawal seed
        withdrawal_seed: u64,
        // Settings address
        settings_address: Pubkey,
        // Sender address
        sender_address: EverAddress,
        // Sender address
        recipient_address: Pubkey,
        // Withdrawal amount
        amount: u64,
    },

    /// Update withdraw status
    ///
    /// # Account references
    /// ...
    UpdateWithdrawStatus {
        // Withdrawal seed
        withdrawal_seed: u64,
    },

    /// Withdraw EVER
    ///
    /// # Account references
    /// ...
    WithdrawEver {
        // Withdrawal seed
        withdrawal_seed: u64,
    },

    /// Withdraw SOL
    ///
    /// # Account references
    /// ...
    WithdrawSol {
        // Withdrawal seed
        withdrawal_seed: u64,
    },

    /// Approve Withdraw Ever
    ///
    /// # Account references
    /// ...
    ApproveWithdrawEver {
        // Withdrawal seed
        withdrawal_seed: u64,
    },

    /// Approve Withdraw SOL
    ///
    /// # Account references
    /// ...
    ApproveWithdrawSol {
        // Withdrawal seed
        withdrawal_seed: u64,
    },

    /// Cancel Withdraw SOL
    ///
    /// # Account references
    /// ...
    CancelWithdrawSol {
        // Withdrawal seed
        withdrawal_seed: u64,
        // Deposit seed
        deposit_seed: u64,
        // Settings address
        settings_address: Pubkey,
    },
    /// Force Withdraw SOL
    ///
    /// # Account references
    /// ...
    ForceWithdrawSol {
        // Withdrawal seed
        withdrawal_seed: u64,
    },

    /// Fill Withdraw SOL
    ///
    /// # Account references
    /// ...
    /**/
    FillWithdrawSol {
        // Withdrawal seed
        withdrawal_seed: u64,
        // Deposit seed
        deposit_seed: u64,
        // Settings address
        settings_address: Pubkey,
        // Recipient address
        recipient_address: EverAddress,
    },

    /// Transfer from Vault
    ///
    /// # Account references
    /// ...
    TransferFromVault {
        // Amount to transfer
        amount: u64,
    },

    /// Change Bounty for Withdraw SOL
    ///
    /// # Account references
    /// ...
    ChangeBountyForWithdrawSol {
        // Withdrawal seed
        withdrawal_seed: u64,
        // Settings address
        settings_address: Pubkey,
        // New bounty value
        bounty: u64,
    },

    /// Change Settings
    ///
    /// # Account references
    /// ...
    ChangeSettings {
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
