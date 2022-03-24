use std::str::FromStr;

use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use solana_program::hash::Hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use crate::{
    get_associated_mint_address, get_associated_settings_address, get_associated_vault_address,
    get_associated_withdrawal_address, get_program_data_address, id, TokenKind,
    TokenProxyInstruction, WithdrawalEvent, WithdrawalMeta,
};

#[wasm_bindgen(js_name = "initializeMint")]
pub fn initialize_mint_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: String,
) -> JsValue {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).unwrap();
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).unwrap();
    let admin = Pubkey::from_str(admin.as_str()).unwrap();

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

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "initializeVault")]
pub fn initialize_vault_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    mint_pubkey: String,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: String,
) -> JsValue {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).unwrap();
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).unwrap();
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).unwrap();
    let admin = Pubkey::from_str(admin.as_str()).unwrap();

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

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "approveWithdrawalEver")]
pub fn approve_withdrawal_ever_ix(
    authority_pubkey: String,
    to_pubkey: String,
    name: String,
    payload_id: String,
) -> JsValue {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).unwrap();
    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).unwrap();
    let payload_id = Hash::from_str(payload_id.as_str()).unwrap();

    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let mint_pubkey = get_associated_mint_address(&name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver { name, payload_id }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "approveWithdrawalSol")]
pub fn approve_withdrawal_sol_ix(
    authority_pubkey: String,
    name: String,
    payload_id: String,
) -> JsValue {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).unwrap();
    let payload_id = Hash::from_str(payload_id.as_str()).unwrap();

    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey = get_associated_withdrawal_address(&payload_id);

    let data = TokenProxyInstruction::ApproveWithdrawSol { name, payload_id }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "confirmWithdrawRequest")]
pub fn confirm_withdraw_request_ix(
    authority_pubkey: String,
    name: String,
    payload_id: String,
    round_number: u32,
) -> JsValue {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).unwrap();
    let payload_id = Hash::from_str(payload_id.as_str()).unwrap();

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

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> JsValue {
    let settings = crate::Settings::unpack(&data).unwrap();

    let s = Settings {
        is_initialized: settings.is_initialized,
        kind: settings.kind,
        admin: settings.admin,
        decimals: settings.decimals,
        emergency: settings.emergency,
        deposit_limit: settings.deposit_limit,
        withdrawal_limit: settings.withdrawal_limit,
        withdrawal_daily_limit: settings.withdrawal_daily_limit,
        withdrawal_daily_amount: settings.withdrawal_daily_amount,
        withdrawal_ttl: settings.withdrawal_ttl,
    };

    return JsValue::from_serde(&s).unwrap();
}

#[wasm_bindgen(js_name = "unpackWithdrawal")]
pub fn unpack_withdrawal(data: Vec<u8>) -> JsValue {
    let withdrawal = crate::Withdrawal::unpack(&data).unwrap();

    let w = Withdrawal {
        is_initialized: withdrawal.is_initialized,
        payload_id: withdrawal.payload_id,
        round_number: withdrawal.round_number,
        required_votes: withdrawal.required_votes,
        signers: withdrawal.signers,
        event: withdrawal.event,
        meta: withdrawal.meta,
    };

    return JsValue::from_serde(&w).unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub is_initialized: bool,
    pub kind: TokenKind,
    pub admin: Pubkey,
    pub decimals: u8,
    pub emergency: bool,
    pub deposit_limit: u64,
    pub withdrawal_limit: u64,
    pub withdrawal_daily_limit: u64,
    pub withdrawal_daily_amount: u64,
    pub withdrawal_ttl: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Withdrawal {
    pub is_initialized: bool,
    pub payload_id: Hash,
    pub round_number: u32,
    pub required_votes: u32,
    pub signers: Vec<Pubkey>,
    pub event: WithdrawalEvent,
    pub meta: WithdrawalMeta,
}
