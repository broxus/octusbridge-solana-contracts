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

    /// Initialize Token Proxy
    ///
    /// # Account references
    /// ...
    Initialize {
        // Guardian pubkey
        guardian: Pubkey,
        // Withdrawal manager pubkey
        withdrawal_manager: Pubkey,
    },

    /// Initialize Mint Account
    ///
    /// # Account references
    /// ...
    InitializeMint {
        // Mint asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        ever_decimals: u8,
        /// Number of base 10 digits to the right of the decimal place.
        solana_decimals: u8,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Withdrawal daily limit
        withdrawal_daily_limit: u64,
    },

    /// Initialize Vault Account
    ///
    /// # Account references
    /// ...
    InitializeVault {
        // Vault asset name
        name: String,
        /// Number of base 10 digits to the right of the decimal place.
        ever_decimals: u8,
        // Deposit limit
        deposit_limit: u64,
        // Withdrawal limit
        withdrawal_limit: u64,
        // Withdrawal daily limit
        withdrawal_daily_limit: u64,
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
        amount: u128,
    },

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
        // Recipient address
        recipient_address: Option<EverAddress>,
    },

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

    /// Change Bounty for Withdraw SOL
    ///
    /// # Account references
    /// ...
    ChangeBountyForWithdrawSol {
        // New bounty value
        bounty: u64,
    },

    /// Change Guardian
    ///
    /// # Account references
    /// ...
    ChangeGuardian {
        // New guardian pubkey
        new_guardian: Pubkey,
    },

    /// Change Withdrawal Manager
    ///
    /// # Account references
    /// ...
    ChangeWithdrawalManager {
        // New withdrawal manager pubkey
        new_withdrawal_manager: Pubkey,
    },

    /// Change deposit limit
    ///
    /// # Account references
    /// ...
    ChangeDepositLimit {
        // Deposit limit
        new_deposit_limit: u64,
    },

    /// Change withdrawal limits
    ///
    /// # Account references
    /// ...
    ChangeWithdrawalLimits {
        // Withdrawal limit
        new_withdrawal_limit: Option<u64>,
        // Withdrawal daily limit
        new_withdrawal_daily_limit: Option<u64>,
    },

    /// Enable emergency mode
    ///
    /// # Account references
    /// ...
    EnableEmergencyMode,

    /// Disable emergency mode
    ///
    /// # Account references
    /// ...
    DisableEmergencyMode,

    /// Enable token emergency mode
    ///
    /// # Account references
    /// ...
    EnableTokenEmergencyMode,

    /// Disable token emergency mode
    ///
    /// # Account references
    /// ...
    DisableTokenEmergencyMode,

    /// Close a new withdrawal account
    ///
    /// # Account references
    /// ...
    CloseWithdrawalAccount,
}
