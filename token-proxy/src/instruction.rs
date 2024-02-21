use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::{EverAddress, UInt256, Vote};

use solana_program::pubkey::Pubkey;

use crate::FeeType;

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

    /// Withdraw Multi Token EVER
    ///
    /// # Account references
    /// ...
    WithdrawMultiTokenEver,

    /// Withdraw Multi Token SOL
    ///
    /// # Account references
    /// ...
    WithdrawMultiTokenSol,

    /// Execute Payload EVER
    ///
    /// # Account references
    /// ...
    ExecutePayloadEver,

    /// Execute Payload SOL
    ///
    /// # Account references
    /// ...
    ExecutePayloadSol,

    /// Initialize Token Proxy
    ///
    /// # Account references
    /// ...
    Initialize {
        // Guardian pubkey
        guardian: Pubkey,
        // Manager pubkey
        manager: Pubkey,
        // Withdrawal manager pubkey
        withdrawal_manager: Pubkey,
    },

    /// Deposit Multi token EVER
    ///
    /// # Account references
    /// ...
    DepositMultiTokenEver {
        // Deposit seed
        deposit_seed: u128,
        // Deposit amount
        amount: u64,
        // Ever recipient address
        recipient: EverAddress,
        // Sol amount to transfer to ever
        value: u64,
        // Expected SOL amount in EVER
        expected_evers: UInt256,
        // Random payload to transfer to ever
        payload: Vec<u8>,
    },

    /// Deposit Multi token SOL
    ///
    /// # Account references
    /// ...
    DepositMultiTokenSol {
        // Deposit seed
        deposit_seed: u128,
        // Mint name
        name: String,
        // Mint symbol
        symbol: String,
        // Deposit amount
        amount: u64,
        // Ever recipient address
        recipient: EverAddress,
        // Sol amount to transfer to ever
        value: u64,
        // Expected SOL amount in EVER
        expected_evers: UInt256,
        // Random payload to transfer to ever
        payload: Vec<u8>,
    },

    /// Withdraw Multi token EVER request
    ///
    /// # Account references
    /// ...
    WithdrawMultiTokenEverRequest {
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
        // Ever token root address
        token: EverAddress,
        // token name
        name: String,
        // token symbol
        symbol: String,
        // decimals
        decimals: u8,
        // Solana recipient address
        recipient: Pubkey,
        // Withdrawal amount
        amount: u128,
        // Random payload to transfer to sol
        payload: Vec<u8>,
        // Attached SOL amount to proxy account
        attached_amount: u64,
    },

    /// Withdraw multi token SOL request
    ///
    /// # Account references
    /// ...
    WithdrawMultiTokenSolRequest {
        // Ever event timestamp
        event_timestamp: u32,
        // Ever event transaction lt
        event_transaction_lt: u64,
        // Ever event configuration
        event_configuration: Pubkey,
        // Solana recipient address
        recipient: Pubkey,
        // Withdrawal amount
        amount: u128,
        // Random payload to transfer to sol
        payload: Vec<u8>,
        // Attached SOL amount to proxy account
        attached_amount: u64,
    },

    /// Change Guardian Role
    ///
    /// # Account references
    /// ...
    ChangeGuardian {
        // New guardian pubkey
        new_guardian: Pubkey,
    },

    /// Change Manager Role
    ///
    /// # Account references
    /// ...
    ChangeManager {
        // New guardian pubkey
        new_manager: Pubkey,
    },

    /// Change Withdrawal Manager Role
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

    /// Update fee
    ///
    /// # Account references
    /// ...
    UpdateFee {
        // Fee type
        fee_type: FeeType,
        // Fee multiplier
        multiplier: u64,
        // Fee divisor
        divisor: u64,
    },

    /// Update token naming
    ///
    /// # Account references
    /// ...
    UpdateTokenName {
        // Token symbol
        symbol: String,
        // Token name
        name: String,
    },

    /// Withdraw EVER fee
    ///
    /// # Account references
    /// ...
    WithdrawEverFee {
        // Amount to withdraw
        amount: u64,
    },

    /// Withdraw SOL fee
    ///
    /// # Account references
    /// ...
    WithdrawSolFee {
        // Amount to withdraw
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

    /// Cancel Withdraw SOL
    ///
    /// # Account references
    /// ...
    CancelWithdrawSol {
        // Deposit seed
        deposit_seed: u128,
        // Recipient address
        recipient: EverAddress,
        // Sol amount to transfer to ever
        value: u64,
        // Expected SOL amount in EVER
        expected_evers: UInt256,
        // Random payload to transfer to ever
        payload: Vec<u8>,
    },

    /// Fill Withdraw SOL
    ///
    /// # Account references
    /// ...
    FillWithdrawSol {
        // Deposit seed
        deposit_seed: u128,
        // Recipient address
        recipient: EverAddress,
        // Deposit amount
        amount: u64,
        // Sol amount to transfer to ever
        value: u64,
        // Expected SOL amount in EVER
        expected_evers: UInt256,
        // Random payload to transfer to ever
        payload: Vec<u8>,
    },

    /// Withdraw tokens from Proxy Account
    ///
    /// # Account references
    /// ...
    WithdrawProxy {
        // Amount to withdraw
        amount: u64,
    },

    /// Close Deposit Account to return SOL
    ///
    /// # Account references
    /// ...
    CloseDeposit,

    /// Close Withdrawal Account to return SOL
    ///
    /// # Account references
    /// ...
    CloseWithdrawal,

    /// Withdraw SOL
    ///
    /// # Account references
    /// ...
    WithdrawMultiVault {
        // Amount SOL to withdraw
        amount: u64,
    },
}
