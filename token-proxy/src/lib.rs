use borsh::BorshSerialize;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::sysvar::rent;

mod error;
mod instruction;
mod processor;
mod state;

pub use self::error::*;
pub use self::instruction::*;
pub use self::processor::*;
pub use self::state::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("TokenProxyPubKey111111111111111111111111111");

pub fn get_associated_settings_address(token_name: &str) -> Pubkey {
    Pubkey::find_program_address(&[b"settings", token_name.as_bytes()], &id()).0
}

pub fn initialize(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
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
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new_readonly(id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}
