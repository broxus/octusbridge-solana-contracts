use borsh::BorshSerialize;
use bridge_utils::types::{EverAddress, Vote};

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use crate::*;

pub fn get_programdata_address() -> Pubkey {
    let program_id = &id();
    bridge_utils::helper::get_programdata_address(program_id)
}

pub fn get_settings_address(name: &str) -> Pubkey {
    let program_id = &id();
    get_associated_settings_address(program_id, name)
}

pub fn get_withdrawal_address(
    author: &Pubkey,
    settings: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
) -> Pubkey {
    let program_id = &id();
    bridge_utils::helper::get_associated_proposal_address(
        program_id,
        author,
        settings,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
    )
}

pub fn get_deposit_address(seed: u128, settings_address: &Pubkey) -> Pubkey {
    let program_id = &id();
    get_associated_deposit_address(program_id, seed, settings_address)
}

pub fn get_vault_address(name: &str) -> Pubkey {
    let program_id = &id();
    get_associated_vault_address(program_id, name)
}

pub fn get_mint_address(name: &str) -> Pubkey {
    let program_id = &id();
    get_associated_mint_address(program_id, name)
}

#[allow(clippy::too_many_arguments)]
pub fn initialize_mint_ix(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: Pubkey,
) -> Instruction {
    let mint_pubkey = get_mint_address(&name);
    let settings_pubkey = get_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

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
pub fn initialize_vault_ix(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    name: String,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: Pubkey,
) -> Instruction {
    let vault_pubkey = get_vault_address(&name);
    let settings_pubkey = get_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::InitializeVault {
        name,
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

pub fn deposit_ever_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    token_name: &str,
    deposit_seed: u128,
    recipient_address: EverAddress,
    amount: u64,
) -> Instruction {
    let mint_pubkey = get_mint_address(token_name);
    let settings_pubkey = get_settings_address(token_name);
    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(author_pubkey, &mint_pubkey);
    let deposit_pubkey = get_deposit_address(deposit_seed, &settings_pubkey);

    let data = TokenProxyInstruction::DepositEver {
        deposit_seed,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
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

pub fn deposit_sol_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    token_name: &str,
    deposit_seed: u128,
    recipient_address: EverAddress,
    amount: u64,
) -> Instruction {
    let vault_pubkey = get_vault_address(token_name);
    let settings_pubkey = get_settings_address(token_name);

    let deposit_pubkey = get_deposit_address(deposit_seed, &settings_pubkey);
    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(author_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::DepositSol {
        deposit_seed,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
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
pub fn withdrawal_request_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    settings_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    round_number: u32,
    sender_address: EverAddress,
    recipient_address: Pubkey,
    amount: u64,
) -> Instruction {
    let withdrawal_pubkey = get_withdrawal_address(
        author_pubkey,
        settings_pubkey,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
    );
    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = TokenProxyInstruction::WithdrawRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        sender_address,
        recipient_address,
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
            AccountMeta::new_readonly(*settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn vote_for_withdrawal_request_ix(
    voter_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = TokenProxyInstruction::VoteForWithdrawRequest { vote }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*voter_pubkey, true),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    }
}

pub fn update_withdrawal_status_ix(
    withdrawal_pubkey: &Pubkey,
    settings_pubkey: &Pubkey,
) -> Instruction {
    let data = TokenProxyInstruction::UpdateWithdrawStatus
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*settings_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn withdrawal_ever_ix(
    to_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address(token_name);
    let mint_pubkey = get_mint_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawEver
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn withdrawal_sol_ix(
    to_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address(token_name);

    let vault_pubkey = get_vault_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::WithdrawSol
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_ever_ix(
    admin_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address(token_name);

    let mint_pubkey = get_mint_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*admin_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_sol_ix(
    admin_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address(token_name);

    let data = TokenProxyInstruction::ApproveWithdrawSol
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*admin_pubkey, true),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    }
}

pub fn cancel_withdrawal_sol_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    deposit_seed: u128,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address(token_name);
    let deposit_pubkey = get_deposit_address(deposit_seed, &settings_pubkey);

    let data = TokenProxyInstruction::CancelWithdrawSol { deposit_seed }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn force_withdrawal_sol_ix(
    to_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    name: String,
) -> Instruction {
    let settings_pubkey = get_settings_address(&name);

    let vault_account = get_vault_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::ForceWithdrawSol
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_account, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fill_withdrawal_sol_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
    deposit_seed: u128,
    recipient_address: EverAddress,
) -> Instruction {
    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(author_pubkey, mint_pubkey);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let settings_pubkey = get_settings_address(token_name);
    let new_deposit_pubkey = get_deposit_address(deposit_seed, &settings_pubkey);

    let data = TokenProxyInstruction::FillWithdrawSol {
        deposit_seed,
        recipient_address,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(new_deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn transfer_from_vault_ix(
    admin_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    amount: u64,
) -> Instruction {
    let settings_pubkey = get_settings_address(&name);

    let vault_pubkey = get_vault_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::TransferFromVault { amount }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*admin_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

pub fn change_bounty_for_withdrawal_sol_ix(
    author_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    bounty: u64,
) -> Instruction {
    let data = TokenProxyInstruction::ChangeBountyForWithdrawSol { bounty }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(*withdrawal_pubkey, false),
        ],
        data,
    }
}

pub fn change_settings_ix(
    authority_pubkey: &Pubkey,
    name: String,
    emergency: bool,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Instruction {
    let settings_pubkey = get_settings_address(&name);

    let data = TokenProxyInstruction::ChangeSettings {
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
