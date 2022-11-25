use borsh::{BorshDeserialize, BorshSerialize};
use bridge_derive::BridgePack;
use bridge_utils::state::{AccountKind, PDA};
use bridge_utils::types::{EverAddress, Vote};
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 32;

pub const WITHDRAWAL_TOKEN_PERIOD: i64 = 86400;

const WITHDRAWAL_MULTI_TOKEN_EVER_EVENT_LEN: usize =
    1 + 1 + PUBKEY_BYTES                      // ever token root address
    + 1                                       // decimals
    + 16                                      // amount
    + PUBKEY_BYTES                            // solana recipient address
;

const WITHDRAWAL_MULTI_TOKEN_SOL_EVENT_LEN: usize =
    PUBKEY_BYTES                              // solana mint address
    + 16                                      // amount
    + PUBKEY_BYTES                            // solana recipient address
;

const WITHDRAWAL_TOKEN_META_LEN: usize = 1  // status
    + 8                                     // bounty
    + 8                                     // epoch
;

const DEPOSIT_MULTI_TOKEN_SOL_EVENT_LEN: usize = PUBKEY_BYTES   // solana mint address
    + 1                                                         // decimals
    + 16                                                        // amount
    + 8                                                         // sol amount
    + 1 + 1 + PUBKEY_BYTES                                      // ever recipient address
;

const DEPOSIT_MULTI_TOKEN_EVER_EVENT_LEN: usize =
    1 + 1 + PUBKEY_BYTES                                    // ever token root address
    + 16                                                    // amount
    + 8                                                     // sol amount
    + 1 + 1 + PUBKEY_BYTES                                  // ever recipient address
;

const DEPOSIT_TOKEN_META_LEN: usize = 16    // seed
;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct Settings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub emergency: bool,
    pub guardian: Pubkey,
    pub manager: Pubkey,
    pub withdrawal_manager: Pubkey,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct MultiVault {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
}

impl Sealed for MultiVault {}

impl IsInitialized for MultiVault {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct TokenSettings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub kind: TokenKind,
    pub deposit_limit: u64,
    pub withdrawal_limit: u64,
    pub withdrawal_daily_limit: u64,
    pub withdrawal_daily_amount: u64,
    pub withdrawal_epoch: i64,
    pub emergency: bool,
    // TODO: fee/amount
}

impl Sealed for TokenSettings {}

impl IsInitialized for TokenSettings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Deposit {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: Vec<u8>,
    pub meta: Vec<u8>,
}

impl Deposit {
    pub fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        let (left, _) = dst.split_at_mut(data.len());
        left.copy_from_slice(&data);
    }

    pub fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::deserialize(&mut src)?;
        Ok(unpacked)
    }
}

impl Sealed for Deposit {}

impl IsInitialized for Deposit {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct DepositMultiTokenSol {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: DepositMultiTokenSolEventWithLen,
    pub meta: DepositTokenMetaWithLen,
}

impl Sealed for DepositMultiTokenSol {}

impl IsInitialized for DepositMultiTokenSol {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositMultiTokenSolEvent {
    pub base_token: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub amount: u128,
    pub sol_amount: u64,
    pub recipient: EverAddress,
    pub payload: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositMultiTokenSolEventWithLen {
    pub len: u32,
    pub data: DepositMultiTokenSolEvent,
}

impl DepositMultiTokenSolEventWithLen {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_token: Pubkey,
        name: String,
        symbol: String,
        decimals: u8,
        amount: u128,
        sol_amount: u64,
        recipient: EverAddress,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            len: (DEPOSIT_MULTI_TOKEN_SOL_EVENT_LEN as u32)
                + 4
                + (name.len() as u32)
                + 4
                + (symbol.len() as u32)
                + 4
                + (payload.len() as u32),
            data: DepositMultiTokenSolEvent {
                base_token,
                name,
                symbol,
                decimals,
                amount,
                sol_amount,
                recipient,
                payload,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct DepositMultiTokenEver {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: DepositMultiTokenEverEventWithLen,
    pub meta: DepositTokenMetaWithLen,
}

impl Sealed for DepositMultiTokenEver {}

impl IsInitialized for DepositMultiTokenEver {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositMultiTokenEverEvent {
    pub token: EverAddress,
    pub amount: u128,
    pub sol_amount: u64,
    pub recipient: EverAddress,
    pub payload: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositMultiTokenEverEventWithLen {
    pub len: u32,
    pub data: DepositMultiTokenEverEvent,
}

impl DepositMultiTokenEverEventWithLen {
    pub fn new(
        token: EverAddress,
        amount: u128,
        sol_amount: u64,
        recipient: EverAddress,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            len: (DEPOSIT_MULTI_TOKEN_EVER_EVENT_LEN + 4 + payload.len()) as u32,
            data: DepositMultiTokenEverEvent {
                token,
                amount,
                sol_amount,
                recipient,
                payload,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositTokenMeta {
    pub seed: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositTokenMetaWithLen {
    pub len: u32,
    pub data: DepositTokenMeta,
}

impl DepositTokenMetaWithLen {
    pub fn new(seed: u128) -> Self {
        Self {
            len: DEPOSIT_TOKEN_META_LEN as u32,
            data: DepositTokenMeta { seed },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct WithdrawalMultiTokenEver {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub is_executed: bool,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: WithdrawalMultiTokenEverEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

impl Sealed for WithdrawalMultiTokenEver {}

impl IsInitialized for WithdrawalMultiTokenEver {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalMultiTokenEverEvent {
    pub token: EverAddress,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub amount: u128,
    pub recipient: Pubkey,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalMultiTokenEverEventWithLen {
    pub len: u32,
    pub data: WithdrawalMultiTokenEverEvent,
}

impl WithdrawalMultiTokenEverEventWithLen {
    pub fn new(
        token: EverAddress,
        name: String,
        symbol: String,
        decimals: u8,
        amount: u128,
        recipient: Pubkey,
    ) -> Self {
        Self {
            len: WITHDRAWAL_MULTI_TOKEN_EVER_EVENT_LEN as u32
                + 4
                + name.as_bytes().len() as u32
                + 4
                + symbol.as_bytes().len() as u32,
            data: WithdrawalMultiTokenEverEvent {
                token,
                name,
                symbol,
                decimals,
                amount,
                recipient,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct WithdrawalMultiTokenSol {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub is_executed: bool,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: WithdrawalMultiTokenSolEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

impl Sealed for WithdrawalMultiTokenSol {}

impl IsInitialized for WithdrawalMultiTokenSol {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalMultiTokenSolEvent {
    pub mint: Pubkey,
    pub amount: u128,
    pub recipient: Pubkey,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalMultiTokenSolEventWithLen {
    pub len: u32,
    pub data: WithdrawalMultiTokenSolEvent,
}

impl WithdrawalMultiTokenSolEventWithLen {
    pub fn new(mint: Pubkey, amount: u128, recipient: Pubkey) -> Self {
        Self {
            len: WITHDRAWAL_MULTI_TOKEN_SOL_EVENT_LEN as u32,
            data: WithdrawalMultiTokenSolEvent {
                mint,
                amount,
                recipient,
            },
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalTokenMeta {
    pub status: WithdrawalTokenStatus,
    pub bounty: u64,
    pub epoch: i64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalTokenMetaWithLen {
    pub len: u32,
    pub data: WithdrawalTokenMeta,
}

impl WithdrawalTokenMetaWithLen {
    pub fn new(status: WithdrawalTokenStatus, bounty: u64, epoch: i64) -> Self {
        Self {
            len: WITHDRAWAL_TOKEN_META_LEN as u32,
            data: WithdrawalTokenMeta {
                status,
                bounty,
                epoch,
            },
        }
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    EnumAsInner,
    PartialEq,
    Eq,
)]
pub enum TokenKind {
    Ever {
        mint: Pubkey,
        token: EverAddress,
        decimals: u8,
    },
    Solana {
        mint: Pubkey,
        vault: Pubkey,
    },
}

#[derive(
    Copy, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, Eq, PartialEq,
)]
pub enum WithdrawalTokenStatus {
    New,
    Processed,
    Cancelled,
    Pending,
    WaitingForApprove,
}
