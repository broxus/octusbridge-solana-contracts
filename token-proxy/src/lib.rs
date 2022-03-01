use borsh::BorshSerialize;
use solana_program::hash::Hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{bpf_loader_upgradeable, system_program, sysvar};
use spl_associated_token_account::get_associated_token_address;

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

solana_program::declare_id!("9pLaxnRNgMQY4Wpk9X1EjBVANwEPjwZw36ok8Af6gW1L");

pub fn get_associated_vault_address(name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"vault", name.as_bytes()], &id()).0
}

pub fn get_associated_mint_address(name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"mint", name.as_bytes()], &id()).0
}

pub fn get_associated_settings_address(name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"settings", name.as_bytes()], &id()).0
}

pub fn get_associated_deposit_address(payload_id: &Hash) -> Pubkey {
    Pubkey::find_program_address(&[br"deposit", &payload_id.to_bytes()], &id()).0
}

pub fn get_associated_withdrawal_address(payload_id: &Hash) -> Pubkey {
    Pubkey::find_program_address(&[br"withdrawal", &payload_id.to_bytes()], &id()).0
}

pub fn get_program_data_address() -> Pubkey {
    Pubkey::find_program_address(&[id().as_ref()], &bpf_loader_upgradeable::id()).0
}

pub fn initialize_mint(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    name: String,
    decimals: u8,
) -> Instruction {
    let mint_pubkey = get_associated_mint_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);
    let program_data_pubkey = get_program_data_address();

    let data = TokenProxyInstruction::InitializeMint { name, decimals }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn initialize_vault(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
) -> Instruction {
    let vault_pubkey = get_associated_vault_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);
    let program_data_pubkey = get_program_data_address();

    let data = TokenProxyInstruction::InitializeVault {
        name,
        deposit_limit,
        withdrawal_limit,
        decimals,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(*mint_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn deposit_ever(
    funder_pubkey: &Pubkey,
    sender_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    let mint_pubkey = get_associated_mint_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);
    let sender_token_pubkey = get_associated_token_address(sender_pubkey, &mint_pubkey);
    let deposit_pubkey = get_associated_deposit_address(&payload_id);

    let data = TokenProxyInstruction::DepositEver {
        name,
        payload_id,
        recipient,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*sender_pubkey, true),
            AccountMeta::new(sender_token_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn deposit_sol(
    funder_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    sender_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    let vault_pubkey = get_associated_vault_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);

    let deposit_pubkey = get_associated_deposit_address(&payload_id);
    let sender_token_pubkey = get_associated_token_address(sender_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::DepositSol {
        name,
        payload_id,
        recipient,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*sender_pubkey, true),
            AccountMeta::new(sender_token_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(*mint_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn withdrawal_ever(
    funder_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
    round_number: u32,
    amount: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);
    let relay_round_pubkey = round_loader::get_associated_relay_round_address(round_number);

    let data = TokenProxyInstruction::WithdrawEver {
        name,
        payload_id,
        round_number,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn confirm_withdrawal_ever(
    relay_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
    round_number: u32,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);
    let relay_round_pubkey = round_loader::get_associated_relay_round_address(round_number);

    let data = TokenProxyInstruction::ConfirmWithdrawEver {
        name,
        payload_id,
        round_number,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}
