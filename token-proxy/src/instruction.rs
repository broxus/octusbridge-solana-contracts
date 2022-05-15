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
        deposit_seed: u128,
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
        deposit_seed: u128,
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
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
        // Sender address
        sender_address: EverAddress,
        // Sender address
        recipient_address: Pubkey,
        // Withdrawal amount
        amount: u64,
    },

    /// Withdraw EVER
    ///
    /// # Account references
    /// ...
    WithdrawEver,

    /// Withdraw SOL
    ///
    /// # Account references
    /// ...
    WithdrawSol,

    /// Approve Withdraw Ever
    ///
    /// # Account references
    /// ...
    ApproveWithdrawEver,

    /// Approve Withdraw SOL
    ///
    /// # Account references
    /// ...
    ApproveWithdrawSol,

    /// Cancel Withdraw SOL
    ///
    /// # Account references
    /// ...
    CancelWithdrawSol {
        // Deposit seed
        deposit_seed: u128,
    },

    /// Force Withdraw SOL
    ///
    /// # Account references
    /// ...
    ForceWithdrawSol,

    /// Fill Withdraw SOL
    ///
    /// # Account references
    /// ...
    FillWithdrawSol {
        // Deposit seed
        deposit_seed: u128,
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
