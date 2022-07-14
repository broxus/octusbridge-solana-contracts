use std::str::FromStr;

use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use bridge_utils::state::*;
use bridge_utils::types::*;

use bridge_utils::helper::get_associated_relay_round_address;

use crate::*;

#[wasm_bindgen(js_name = "initializeSettings")]
pub fn initialize_settings_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    guardian: String,
    withdrawal_manager: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let guardian = Pubkey::from_str(guardian.as_str()).handle_error()?;
    let withdrawal_manager = Pubkey::from_str(withdrawal_manager.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::Initialize {
        guardian,
        withdrawal_manager,
    }
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "initializeMint")]
pub fn initialize_mint_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    name: String,
    ever_decimals: u8,
    solana_decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Result<JsValue, JsValue> {
    let mint_pubkey = get_mint_address(&name);
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::InitializeMint {
        name,
        ever_decimals,
        solana_decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "initializeVault")]
pub fn initialize_vault_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    mint_pubkey: String,
    name: String,
    ever_decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Result<JsValue, JsValue> {
    let vault_pubkey = get_vault_address(&name);
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::InitializeVault {
        name,
        ever_decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalRequest")]
pub fn withdrawal_request_ix(
    funder_pubkey: String,
    author_pubkey: String,
    name: String,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    sender_address: String,
    recipient_address: String,
    amount: String,
    round_number: u32,
) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = get_token_settings_address(&name);

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let amount = u128::from_str(&amount).handle_error()?;

    let sender_address = EverAddress::from_str(&sender_address).handle_error()?;

    let relay_round_pubkey = get_associated_relay_round_address(&round_loader::id(), round_number);

    let withdrawal_pubkey = get_withdrawal_address(
        &token_settings_pubkey,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    let rl_settings_pubkey = bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let data = TokenProxyInstruction::WithdrawRequest {
        event_timestamp,
        event_transaction_lt,
        sender_address,
        event_configuration,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(rl_settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "approveWithdrawalEver")]
pub fn approve_withdrawal_ever_ix(
    authority_pubkey: String,
    to_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
) -> Result<JsValue, JsValue> {
    let mint_pubkey = get_mint_address(&name);
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "approveWithdrawalEverByOwner")]
pub fn approve_withdrawal_ever_by_owner_ix(
    authority_pubkey: String,
    to_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
) -> Result<JsValue, JsValue> {
    let mint_pubkey = get_mint_address(&name);
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "approveWithdrawalSol")]
pub fn approve_withdrawal_sol_ix(
    authority_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
    to_pubkey: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let mint_pubkey = get_mint_address(&name);
    let vault_pubkey = get_vault_address(&name);

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawSol
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "approveWithdrawalSolByOwner")]
pub fn approve_withdrawal_sol_by_owner_ix(
    authority_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
    to_pubkey: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let mint_pubkey = get_mint_address(&name);
    let vault_pubkey = get_vault_address(&name);
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawSol
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalEver")]
pub fn withdrawal_ever_ix(
    to_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
) -> Result<JsValue, JsValue> {
    let mint_pubkey = get_mint_address(&name);

    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawEver
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalSol")]
pub fn withdrawal_sol_ix(
    to_pubkey: String,
    mint_pubkey: String,
    withdrawal_pubkey: String,
    name: String,
) -> Result<JsValue, JsValue> {
    let vault_pubkey = get_vault_address(&name);

    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawSol
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "cancelWithdrawalSol")]
pub fn cancel_withdrawal_sol_ix(
    funder_pubkey: String,
    author_pubkey: String,
    withdrawal_pubkey: String,
    deposit_seed: String,
    name: String,
    recipient_address: Option<String>,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let recipient_address = recipient_address
        .map(|value| EverAddress::from_str(&value))
        .transpose()
        .handle_error()?;

    let data = TokenProxyInstruction::CancelWithdrawSol {
        deposit_seed,
        recipient_address,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "fillWithdrawalSol")]
pub fn fill_withdrawal_sol_ix(
    funder_pubkey: String,
    author_pubkey: String,
    to_pubkey: String,
    mint_pubkey: String,
    withdrawal_pubkey: String,
    name: String,
    deposit_seed: String,
    recipient_address: String,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let new_deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).handle_error()?;
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let recipient_address = EverAddress::from_str(&recipient_address).handle_error()?;

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&author_pubkey, &mint_pubkey);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::FillWithdrawSol {
        deposit_seed,
        recipient_address,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(new_deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "depositEver")]
pub fn deposit_ever_ix(
    funder_pubkey: String,
    authority_pubkey: String,
    name: String,
    deposit_seed: String,
    recipient_address: String,
    amount: u64,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let mint_pubkey = get_mint_address(&name);

    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let recipient_address = EverAddress::from_str(&recipient_address).handle_error()?;

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&authority_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::DepositEver {
        deposit_seed,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(authority_pubkey, true),
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
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "depositSol")]
pub fn deposit_sol_ix(
    funder_pubkey: String,
    author_pubkey: String,
    mint_pubkey: String,
    name: String,
    deposit_seed: String,
    recipient_address: String,
    amount: u64,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let vault_pubkey = get_vault_address(&name);

    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&name);

    let deposit_pubkey = get_deposit_address(deposit_seed, &token_settings_pubkey);

    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;

    let recipient_address = EverAddress::from_str(&recipient_address).handle_error()?;

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&author_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::DepositSol {
        deposit_seed,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(mint_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "voteForWithdrawRequest")]
pub fn vote_for_withdraw_request_ix(
    authority_pubkey: String,
    withdrawal_pubkey: String,
    round_number: u32,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let relay_round_pubkey = get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = TokenProxyInstruction::VoteForWithdrawRequest {
        vote: Vote::Confirm,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeGuardian")]
pub fn change_guardian_ix(
    authority_pubkey: String,
    new_guardian: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let new_guardian = Pubkey::from_str(new_guardian.as_str()).handle_error()?;

    let data = TokenProxyInstruction::ChangeGuardian { new_guardian }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeWithdrawalManager")]
pub fn change_withdrawal_manager_ix(
    authority_pubkey: String,
    new_withdrawal_manager: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let new_withdrawal_manager =
        Pubkey::from_str(new_withdrawal_manager.as_str()).handle_error()?;

    let data = TokenProxyInstruction::ChangeWithdrawalManager {
        new_withdrawal_manager,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeBounty")]
pub fn change_bounty_ix(
    author_pubkey: String,
    withdrawal_pubkey: String,
    bounty: u64,
) -> Result<JsValue, JsValue> {
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::ChangeBountyForWithdrawSol { bounty }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeDepositLimit")]
pub fn change_deposit_limit_ix(
    authority_pubkey: String,
    name: String,
    new_deposit_limit: u64,
) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::ChangeDepositLimit { new_deposit_limit }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeWithdrawalLimits")]
pub fn change_withdrawal_limits_ix(
    authority_pubkey: String,
    name: String,
    new_withdrawal_limit: Option<u64>,
    new_withdrawal_daily_limit: Option<u64>,
) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = get_token_settings_address(&name);
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::ChangeWithdrawalLimits {
        new_withdrawal_limit,
        new_withdrawal_daily_limit,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableEmergency")]
pub fn enable_emergency_ix(authority_pubkey: String) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::EnableEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableEmergencyByOwner")]
pub fn enable_emergency_by_owner_ix(
    authority_pubkey: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::EnableEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "disableEmergency")]
pub fn disable_emergency_ix(authority_pubkey: String) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::DisableEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).handle_error();
}

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let settings = Settings::unpack(&data).handle_error()?;

    let s = WasmSettings {
        emergency: settings.emergency,
        guardian: settings.guardian,
        withdrawal_manager: settings.withdrawal_manager,
    };

    return JsValue::from_serde(&s).handle_error();
}

#[wasm_bindgen(js_name = "unpackTokenSettings")]
pub fn unpack_token_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let token_settings = TokenSettings::unpack(&data).handle_error()?;

    let s = WasmTokenSettings {
        is_initialized: token_settings.is_initialized,
        account_kind: token_settings.account_kind,
        name: token_settings.name,
        ever_decimals: token_settings.ever_decimals,
        solana_decimals: token_settings.solana_decimals,
        kind: token_settings.kind,
        deposit_limit: token_settings.deposit_limit,
        withdrawal_limit: token_settings.withdrawal_limit,
        withdrawal_daily_limit: token_settings.withdrawal_daily_limit,
        withdrawal_daily_amount: token_settings.withdrawal_daily_amount,
        withdrawal_ttl: token_settings.withdrawal_ttl,
    };

    return JsValue::from_serde(&s).handle_error();
}

#[wasm_bindgen(js_name = "unpackWithdrawal")]
pub fn unpack_withdrawal(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let withdrawal = WithdrawalToken::unpack(&data).handle_error()?;

    let w = WasmWithdrawalToken {
        is_initialized: withdrawal.is_initialized,
        account_kind: withdrawal.account_kind,
        is_executed: withdrawal.is_executed,
        round_number: withdrawal.round_number,
        required_votes: withdrawal.required_votes,
        pda: withdrawal.pda,
        event: withdrawal.event,
        meta: withdrawal.meta,
        signers: withdrawal.signers,
    };

    return JsValue::from_serde(&w).handle_error();
}

#[wasm_bindgen(js_name = "unpackDeposit")]
pub fn unpack_deposit(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let deposit = DepositToken::unpack(&data).handle_error()?;

    let d = WasmDepositToken {
        is_initialized: deposit.is_initialized,
        account_kind: deposit.account_kind,
        event: deposit.event,
        meta: WasmDepositTokenMeta {
            seed: deposit.meta.data.seed.to_string(),
        },
    };

    return JsValue::from_serde(&d).handle_error();
}

#[derive(Serialize, Deserialize)]
pub struct WasmSettings {
    pub emergency: bool,
    pub guardian: Pubkey,
    pub withdrawal_manager: Pubkey,
}

#[derive(Serialize, Deserialize)]
pub struct WasmTokenSettings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub name: String,
    pub ever_decimals: u8,
    pub solana_decimals: u8,
    pub kind: TokenKind,
    pub deposit_limit: u64,
    pub withdrawal_limit: u64,
    pub withdrawal_daily_limit: u64,
    pub withdrawal_daily_amount: u64,
    pub withdrawal_ttl: i64,
}

#[derive(Serialize, Deserialize)]
pub struct WasmWithdrawalToken {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub is_executed: bool,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: WithdrawalTokenEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositToken {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: DepositTokenEventWithLen,
    pub meta: WasmDepositTokenMeta,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositTokenMeta {
    pub seed: String,
}

impl<T, E> HandleError for Result<T, E>
where
    E: ToString,
{
    type Output = T;

    fn handle_error(self) -> Result<Self::Output, JsValue> {
        self.map_err(|e| {
            let error = e.to_string();
            js_sys::Error::new(&error).unchecked_into()
        })
    }
}

pub trait HandleError {
    type Output;

    fn handle_error(self) -> Result<Self::Output, JsValue>;
}
