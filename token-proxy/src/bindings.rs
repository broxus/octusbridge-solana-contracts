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

pub fn get_token_settings_address(
    name: &str,
    symbol: &str,
    ever_decimals: u8,
    solana_decimals: u8,
    mint: &Pubkey,
) -> Pubkey {
    let program_id = &id();
    get_associated_token_settings_address(
        program_id,
        name,
        symbol,
        ever_decimals,
        solana_decimals,
        mint,
    )
}

pub fn get_multivault_withdrawal_ever_address(
    settings: &Pubkey,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
    token_address: EverAddress,
    name: String,
    symbol: String,
    decimals: u8,
    recipient_address: Pubkey,
    amount: u128,
) -> Pubkey {
    let program_id = &id();

    let event_data = hash(
        &WithdrawalMultiTokenEverEventWithLen::new(
            token_address,
            name,
            symbol,
            decimals,
            amount,
            recipient_address,
        )
        .data
        .try_to_vec()
        .expect("pack"),
    )
    .to_bytes();

    bridge_utils::helper::get_associated_proposal_address(
        program_id,
        settings,
        round_number,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        &event_data,
    )
}

pub fn get_multivault_withdrawal_sol_address(
    settings: &Pubkey,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
    token_address: Pubkey,
    recipient_address: Pubkey,
    amount: u128,
) -> Pubkey {
    let program_id = &id();

    let event_data = hash(
        &WithdrawalMultiTokenSolEventWithLen::new(token_address, amount, recipient_address)
            .data
            .try_to_vec()
            .expect("pack"),
    )
    .to_bytes();

    bridge_utils::helper::get_associated_proposal_address(
        program_id,
        settings,
        round_number,
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

pub fn get_vault_address(
    name: &str,
    symbol: &str,
    ever_decimals: u8,
    solana_decimals: u8,
    mint: &Pubkey,
) -> Pubkey {
    let program_id = &id();
    get_associated_vault_address(
        program_id,
        name,
        symbol,
        ever_decimals,
        solana_decimals,
        mint,
    )
}

pub fn get_multivault_address() -> Pubkey {
    let program_id = &id();
    get_associated_multivault_address(program_id)
}

pub fn get_mint_address(
    name: &str,
    symbol: &str,
    ever_decimals: u8,
    solana_decimals: u8,
) -> Pubkey {
    let program_id = &id();
    get_associated_mint_address(program_id, name, symbol, ever_decimals, solana_decimals)
}

#[allow(clippy::too_many_arguments)]
pub fn initialize_settings_ix(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    guardian: Pubkey,
    withdrawal_manager: Pubkey,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = TokenProxyInstruction::Initialize {
        guardian,
        withdrawal_manager,
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

pub fn deposit_multi_token_ever_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    author_token_pubkey: &Pubkey,
    token_settings_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    deposit_seed: Uuid,
    recipient_address: EverAddress,
    token_address: EverAddress,
    amount: u64,
    sol_amount: u64,
    payload: Vec<u8>,
) -> Instruction {
    let settings_pubkey = get_settings_address();

    let deposit_seed = deposit_seed.as_u128();
    let deposit_pubkey = get_deposit_address(deposit_seed, token_settings_pubkey);

    let multivault_pubkey = get_multivault_address();

    let data = TokenProxyInstruction::DepositMultiTokenEver {
        deposit_seed,
        recipient_address,
        token_address,
        amount,
        sol_amount,
        payload,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(*author_token_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(*mint_pubkey, false),
            AccountMeta::new(multivault_pubkey, false),
            AccountMeta::new(*token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn deposit_multi_token_sol_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    author_token_pubkey: &Pubkey,
    token_settings_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    vault_pubkey: &Pubkey,
    deposit_seed: Uuid,
    recipient_address: EverAddress,
    amount: u64,
    name: String,
    symbol: String,
    sol_amount: u64,
    payload: Vec<u8>,
) -> Instruction {
    let settings_pubkey = get_settings_address();

    let deposit_seed = deposit_seed.as_u128();
    let deposit_pubkey = get_deposit_address(deposit_seed, token_settings_pubkey);

    let multivault_pubkey = get_multivault_address();

    let data = TokenProxyInstruction::DepositMultiTokenSol {
        deposit_seed,
        recipient_address,
        amount,
        name,
        symbol,
        sol_amount,
        payload,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(*author_token_pubkey, false),
            AccountMeta::new(*vault_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(*mint_pubkey, false),
            AccountMeta::new(multivault_pubkey, false),
            AccountMeta::new(*token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn withdrawal_multi_token_ever_request_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    token_settings_pubkey: &Pubkey,
    token_address: EverAddress,
    name: String,
    symbol: String,
    decimals: u8,
    round_number: u32,
    recipient_address: Pubkey,
    amount: u128,
) -> Instruction {
    let withdrawal_pubkey = get_multivault_withdrawal_ever_address(
        &token_settings_pubkey,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token_address,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient_address,
        amount,
    );
    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());
    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = TokenProxyInstruction::WithdrawMultiTokenEverRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        token_address,
        name,
        symbol,
        decimals,
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

#[allow(clippy::too_many_arguments)]
pub fn withdrawal_multi_token_sol_request_ix(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    token_settings_pubkey: &Pubkey,
    token_address: Pubkey,
    round_number: u32,
    recipient_address: Pubkey,
    amount: u128,
) -> Instruction {
    let withdrawal_pubkey = get_multivault_withdrawal_sol_address(
        &token_settings_pubkey,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token_address,
        recipient_address,
        amount,
    );
    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());
    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = TokenProxyInstruction::WithdrawMultiTokenSolRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        token_address,
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
