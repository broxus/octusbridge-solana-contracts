use borsh::BorshSerialize;
use bridge_utils::types::{EverAddress, Vote};
use uuid::Uuid;

use solana_program::hash::hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use crate::*;

pub fn get_programdata_address() -> Pubkey {
    let program_id = &id();
    bridge_utils::helper::get_programdata_address(program_id)
}

pub fn get_settings_address() -> Pubkey {
    let program_id = &id();
    get_associated_settings_address(program_id)
}

pub fn get_token_settings_address(name: &str) -> Pubkey {
    let program_id = &id();
    get_associated_token_settings_address(program_id, name)
}

pub fn get_withdrawal_address(
    settings: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
    sender_address: EverAddress,
    recipient_address: Pubkey,
    amount: u128,
) -> Pubkey {
    let program_id = &id();

    let event_data = hash(
        &WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address)
            .data
            .try_to_vec()
            .expect("pack"),
    )
    .to_bytes();

    bridge_utils::helper::get_associated_proposal_address(
        program_id,
        settings,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        &event_data,
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
pub fn initialize_settings_ix(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    guardian: Pubkey,
    withdrawal_manager: Pubkey,
    proposal_manager: Pubkey,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::Initialize {
        guardian,
        withdrawal_manager,
        proposal_manager,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn initialize_mint_ix(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    name: String,
    ever_decimals: u8,
    solana_decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Instruction {
    let mint_pubkey = get_mint_address(&name);
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::InitializeMint {
        name,
        ever_decimals,
        solana_decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
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
    ever_decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Instruction {
    let vault_pubkey = get_vault_address(&name);
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::InitializeVault {
        name,
        ever_decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
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
            AccountMeta::new(token_settings_pubkey, false),
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
    deposit_seed: Uuid,
    recipient_address: EverAddress,
    amount: u64,
) -> Instruction {
    let mint_pubkey = get_mint_address(token_name);
    let token_settings_pubkey = get_token_settings_address(token_name);
    let settings_pubkey = get_settings_address();
    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(author_pubkey, &mint_pubkey);

    let deposit_seed = deposit_seed.as_u128();
    let deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

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
            AccountMeta::new(token_settings_pubkey, false),
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
    deposit_seed: Uuid,
    recipient_address: EverAddress,
    amount: u64,
) -> Instruction {
    let vault_pubkey = get_vault_address(token_name);
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);

    let deposit_seed = deposit_seed.as_u128();
    let deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);
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
            AccountMeta::new_readonly(token_settings_pubkey, false),
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
    token_settings_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    round_number: u32,
    sender_address: EverAddress,
    recipient_address: Pubkey,
    amount: u128,
) -> Instruction {
    let withdrawal_pubkey = get_withdrawal_address(
        token_settings_pubkey,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );
    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());
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
            AccountMeta::new_readonly(*token_settings_pubkey, false),
            AccountMeta::new_readonly(rl_settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
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

pub fn withdrawal_ever_ix(
    to_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);
    let mint_pubkey = get_mint_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawEver
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
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
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);

    let vault_pubkey = get_vault_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::WithdrawSol
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_ever_ix(
    authority_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);

    let mint_pubkey = get_mint_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_ever_by_owner_ix(
    owner_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);
    let program_data_pubkey = get_programdata_address();

    let mint_pubkey = get_mint_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn approve_withdrawal_sol_ix(
    authority_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);

    let vault_pubkey = get_vault_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawSol
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn approve_withdrawal_sol_by_owner_ix(
    owner_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    token_name: &str,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);
    let program_data_pubkey = get_programdata_address();

    let vault_pubkey = get_vault_address(token_name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawSol
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn cancel_withdrawal_sol_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    withdrawal_pubkey: &Pubkey,
    deposit_seed: Uuid,
    token_name: &str,
    recipient_address: Option<EverAddress>,
) -> Instruction {
    let deposit_seed = deposit_seed.as_u128();
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);
    let deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

    let data = TokenProxyInstruction::CancelWithdrawSol {
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
            AccountMeta::new(*withdrawal_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
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
    deposit_seed: Uuid,
    recipient_address: EverAddress,
) -> Instruction {
    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(author_pubkey, mint_pubkey);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let deposit_seed = deposit_seed.as_u128();
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);
    let new_deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

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
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
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

pub fn change_guardian_ix(owner: &Pubkey, new_guardian: Pubkey) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::ChangeGuardian { new_guardian }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn change_withdrawal_manager_ix(owner: &Pubkey, new_withdrawal_manager: Pubkey) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::ChangeWithdrawalManager {
        new_withdrawal_manager,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn change_deposit_limit_ix(
    owner_pubkey: &Pubkey,
    name: String,
    new_deposit_limit: u64,
) -> Instruction {
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::ChangeDepositLimit { new_deposit_limit }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn change_withdrawal_limits_ix(
    owner_pubkey: &Pubkey,
    name: String,
    new_withdrawal_limit: Option<u64>,
    new_withdrawal_daily_limit: Option<u64>,
) -> Instruction {
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::ChangeWithdrawalLimits {
        new_withdrawal_limit,
        new_withdrawal_daily_limit,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn enable_emergency_ix(guardian_pubkey: &Pubkey) -> Instruction {
    let settings_pubkey = get_settings_address();

    let data = TokenProxyInstruction::EnableEmergencyMode
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*guardian_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
        ],
        data,
    }
}

pub fn enable_emergency_by_owner_ix(owner_pubkey: &Pubkey) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::EnableEmergencyMode
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn disable_emergency_ix(owner_pubkey: &Pubkey) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::DisableEmergencyMode
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn enable_token_emergency_ix(guardian_pubkey: &Pubkey, token_name: &str) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);

    let data = TokenProxyInstruction::EnableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*guardian_pubkey, true),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
        ],
        data,
    }
}

pub fn enable_token_emergency_by_owner_ix(owner_pubkey: &Pubkey, token_name: &str) -> Instruction {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(token_name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::EnableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn disable_token_emergency_ix(owner_pubkey: &Pubkey, token_name: &str) -> Instruction {
    let token_settings_pubkey = get_token_settings_address(token_name);
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::DisableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn close_proposal_account_ix(owner_pubkey: &Pubkey, proposal_pubkey: &Pubkey) -> Instruction {
    let settings_pubkey = get_settings_address();

    let data = TokenProxyInstruction::CloseProposalAccount
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    }
}

pub fn close_proposal_account_by_owner_ix(
    owner_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::CloseProposalAccount
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*owner_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}
