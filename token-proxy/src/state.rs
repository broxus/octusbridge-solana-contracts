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
pub const WITHDRAWAL_TOKEN_PERIOD: i64 = 86400;

const WITHDRAWAL_MULTI_TOKEN_EVER_EVENT_LEN: usize =
    1 + 1 + PUBKEY_BYTES                      // ever token address
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

const DEPOSIT_TOKEN_META_LEN: usize = 16    // seed
;

const DEPOSIT_MULTI_TOKEN_SOL_EVENT_LEN: usize = PUBKEY_BYTES   // solana mint address
    + 16                                                    // amount
    + 8                                                     // sol amount
    + 1 + 1 + PUBKEY_BYTES                                  // ever recipient address
    + 1                                                     // decimals
    + 4                                                     // payload length
    + 4                                                     // name length
    + 4                                                     // symbol length
;

const DEPOSIT_MULTI_TOKEN_EVER_EVENT_LEN: usize =
    1 + 1 + PUBKEY_BYTES                                    // ever token address
    + 16                                                    // amount
    + 8                                                     // sol amount
    + 1 + 1 + PUBKEY_BYTES                                  // ever recipient address
    + 4                                                     // payload length
;

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 1000)]
pub struct Settings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub emergency: bool,
    pub guardian: Pubkey,
    pub withdrawal_manager: Pubkey,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, BridgePack)]
#[bridge_pack(length = 500)]
pub struct TokenSettings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub name: String,
    pub symbol: String,
    pub ever_decimals: u8,
    pub solana_decimals: u8,
    pub kind: TokenKind,
    pub deposit_limit: u64,
    pub withdrawal_limit: u64,
    pub withdrawal_daily_limit: u64,
    pub withdrawal_daily_amount: u64,
    pub withdrawal_epoch: i64,
    pub emergency: bool,
}

impl Sealed for TokenSettings {}

impl IsInitialized for TokenSettings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
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
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub amount: u128,
    pub sol_amount: u64,
    pub recipient_address: EverAddress,
    pub payload: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositMultiTokenSolEventWithLen {
    pub len: u32,
    pub data: DepositMultiTokenSolEvent,
}

impl DepositMultiTokenSolEventWithLen {
    pub fn new(
        mint: Pubkey,
        name: String,
        symbol: String,
        decimals: u8,
        amount: u128,
        sol_amount: u64,
        recipient_address: EverAddress,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            len: (DEPOSIT_MULTI_TOKEN_SOL_EVENT_LEN as u32)
                + (payload.len() as u32)
                + (name.as_bytes().len() as u32)
                + (symbol.as_bytes().len() as u32),
            data: DepositMultiTokenSolEvent {
                mint,
                name,
                symbol,
                decimals,
                amount,
                sol_amount,
                recipient_address,
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
    pub token_address: EverAddress,
    pub amount: u128,
    pub sol_amount: u64,
    pub recipient_address: EverAddress,
    pub payload: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct DepositMultiTokenEverEventWithLen {
    pub len: u32,
    pub data: DepositMultiTokenEverEvent,
}

impl DepositMultiTokenEverEventWithLen {
    pub fn new(
        token_address: EverAddress,
        amount: u128,
        sol_amount: u64,
        recipient_address: EverAddress,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            len: (DEPOSIT_MULTI_TOKEN_EVER_EVENT_LEN + payload.len()) as u32,
            data: DepositMultiTokenEverEvent {
                token_address,
                amount,
                sol_amount,
                recipient_address,
                payload,
            },
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
    pub token_address: EverAddress,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub amount: u128,
    pub recipient_address: Pubkey,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalMultiTokenEverEventWithLen {
    pub len: u32,
    pub data: WithdrawalMultiTokenEverEvent,
}

impl WithdrawalMultiTokenEverEventWithLen {
    pub fn new(
        token_address: EverAddress,
        name: String,
        symbol: String,
        decimals: u8,
        amount: u128,
        recipient_address: Pubkey,
    ) -> Self {
        Self {
            len: WITHDRAWAL_MULTI_TOKEN_EVER_EVENT_LEN as u32
                + name.as_bytes().len() as u32
                + symbol.as_bytes().len() as u32,
            data: WithdrawalMultiTokenEverEvent {
                token_address,
                name,
                symbol,
                decimals,
                amount,
                recipient_address,
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
    pub token_address: Pubkey,
    pub amount: u128,
    pub recipient_address: Pubkey,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct WithdrawalMultiTokenSolEventWithLen {
    pub len: u32,
    pub data: WithdrawalMultiTokenSolEvent,
}

impl WithdrawalMultiTokenSolEventWithLen {
    pub fn new(token_address: Pubkey, amount: u128, recipient_address: Pubkey) -> Self {
        Self {
            len: WITHDRAWAL_MULTI_TOKEN_SOL_EVENT_LEN as u32,
            data: WithdrawalMultiTokenSolEvent {
                token_address,
                amount,
                recipient_address,
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
    Ever { mint: Pubkey },
    Solana { mint: Pubkey, vault: Pubkey },
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
