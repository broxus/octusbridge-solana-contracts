use borsh::BorshSerialize;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::sysvar::rent;

mod error;
mod instruction;
mod processor;
mod state;
mod utils;

pub use self::error::*;
pub use self::instruction::*;
pub use self::processor::*;
pub use self::state::*;
pub use self::utils::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("TokenProxyPubKey111111111111111111111111111");

pub fn get_associated_proposal_address(relay_address: &Pubkey, round: u32) -> Pubkey {
    Pubkey::find_program_address(&[&relay_address.to_bytes(), &round.to_le_bytes()], &id()).0
}

pub fn get_associated_settings_address(token_name: &str) -> Pubkey {
    Pubkey::find_program_address(&[b"settings", token_name.as_bytes()], &id()).0
}

pub fn get_associated_relay_round_address(round: u32) -> Pubkey {
    Pubkey::find_program_address(&[&round.to_le_bytes()], &id()).0
}

pub fn initialize(
    authority_pubkey: &Pubkey,
    program_buffer_pubkey: &Pubkey,
    name: String,
    kind: TokenKind,
    withdrawal_limit: u64,
    deposit_limit: u64,
    decimals: u8,
    admin: Pubkey,
    token: Pubkey,
) -> Instruction {
    let setting_pubkey = get_associated_settings_address(&name);

    let data = TokenProxyInstruction::Initialize {
        name,
        kind,
        withdrawal_limit,
        deposit_limit,
        decimals,
        admin,
        token,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new_readonly(id(), false),
            AccountMeta::new_readonly(*program_buffer_pubkey, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}
