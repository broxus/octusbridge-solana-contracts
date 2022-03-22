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

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
extern crate wasm_bindgen;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("DqqsutJd5sjtP6ZHbscN2bu98u2JRgRyMJGRimffHCv4");

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

#[allow(clippy::too_many_arguments)]
pub fn initialize_mint(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: Pubkey,
) -> Instruction {
    let mint_pubkey = get_associated_mint_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);
    let program_data_pubkey = get_program_data_address();

    let data = TokenProxyInstruction::InitializeMint {
        name,
        decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
    }
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

#[allow(clippy::too_many_arguments)]
pub fn initialize_vault(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: Pubkey,
) -> Instruction {
    let vault_pubkey = get_associated_vault_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);
    let program_data_pubkey = get_program_data_address();

    let data = TokenProxyInstruction::InitializeVault {
        name,
        decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
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
    recipient: EverAddress,
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
    recipient: EverAddress,
    amount: u64,
) -> Instruction {
    let vault_pubkey = get_associated_vault_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);

    let deposit_pubkey = get_associated_deposit_address(&payload_id);
    let sender_token_pubkey = get_associated_token_address(sender_pubkey, mint_pubkey);

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

#[allow(clippy::too_many_arguments)]
pub fn withdrawal_request(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
    round_number: u32,
    sender: EverAddress,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);
    let relay_round_pubkey = round_loader::get_associated_relay_round_address(round_number);

    let mint_pubkey = get_associated_mint_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(&recipient, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawRequest {
        name,
        payload_id,
        round_number,
        sender,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(recipient_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn confirm_withdrawal_request(
    relay_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
    round_number: u32,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);
    let relay_round_pubkey = round_loader::get_associated_relay_round_address(round_number);

    let data = TokenProxyInstruction::ConfirmWithdrawRequest {
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

pub fn withdrawal_ever(to_pubkey: &Pubkey, name: String, payload_id: Hash) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let mint_pubkey = get_associated_mint_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawEver { name, payload_id }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn withdrawal_sol(
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let vault_account = get_associated_vault_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::WithdrawSol { name, payload_id }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_account, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_ever(
    authority_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let mint_pubkey = get_associated_mint_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver { name, payload_id }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_sol(
    authority_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let data = TokenProxyInstruction::ApproveWithdrawSol { name, payload_id }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    }
}

pub fn cancel_withdrawal_sol(
    funder_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    payload_id: Hash,
    deposit_payload_id: Hash,
) -> Instruction {
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);
    let deposit_pubkey = get_associated_deposit_address(&deposit_payload_id);

    let data = TokenProxyInstruction::CancelWithdrawSol {
        payload_id,
        deposit_payload_id,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn force_withdrawal_sol(
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    payload_id: Hash,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let vault_account = get_associated_vault_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::ForceWithdrawSol { name, payload_id }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_account, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn fill_withdrawal_sol(
    funder_pubkey: &Pubkey,
    authority_sender_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    payload_id: Hash,
    deposit_payload_id: Hash,
    recipient: EverAddress,
) -> Instruction {
    let sender_pubkey = spl_associated_token_account::get_associated_token_address(
        authority_sender_pubkey,
        mint_pubkey,
    );
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);
    let new_deposit_pubkey = get_associated_deposit_address(&deposit_payload_id);

    let data = TokenProxyInstruction::FillWithdrawSol {
        payload_id,
        deposit_payload_id,
        recipient,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*authority_sender_pubkey, true),
            AccountMeta::new(sender_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(new_deposit_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn transfer_from_vault(
    authority_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    amount: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);

    let vault_pubkey = get_associated_vault_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::TransferFromVault { name, amount }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn change_bounty_for_withdrawal_sol(
    authority_pubkey: &Pubkey,
    payload_id: Hash,
    bounty: u64,
) -> Instruction {
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let data = TokenProxyInstruction::ChangeBountyForWithdrawSol { payload_id, bounty }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
        ],
        data,
    }
}

pub fn change_settings(
    authority_pubkey: &Pubkey,
    name: String,
    emergency: bool,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);

    let data = TokenProxyInstruction::ChangeSettings {
        name,
        emergency,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
        ],
        data,
    }
}
