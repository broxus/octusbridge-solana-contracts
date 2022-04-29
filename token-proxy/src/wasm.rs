use std::str::FromStr;

use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use bridge_utils::helper::*;
use bridge_utils::state::*;
use bridge_utils::types::*;

use crate::*;

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
    let program_id = &id();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).unwrap();
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).unwrap();
    let admin = Pubkey::from_str(admin.as_str()).unwrap();

    let mint_pubkey = get_associated_mint_address(program_id, &name);
    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let program_data_pubkey = bridge_utils::helper::get_programdata_address(program_id);

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
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: String,
) -> JsValue {
    let program_id = &id();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).unwrap();
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).unwrap();
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).unwrap();
    let admin = Pubkey::from_str(admin.as_str()).unwrap();

    let vault_pubkey = get_associated_vault_address(program_id, &name);
    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let program_data_pubkey = bridge_utils::helper::get_programdata_address(program_id);

    let data = TokenProxyInstruction::InitializeVault {
        name,
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

#[wasm_bindgen(js_name = "processWithdrawRequest")]
pub fn process_withdraw_request(
    author_pubkey: String,
    name: String,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    sender_address: String,
    recipient_address: String,
    amount: u64,
    round_number: u32,
) -> JsValue {
    let program_id = &id();

    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).unwrap();
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).unwrap();
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).unwrap();
    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let relay_round_pubkey = get_associated_relay_round_address(&round_loader::id(), round_number);
    let sender_address = sender_address.as_bytes().try_into().unwrap();

    let withdrawal_pubkey = get_associated_proposal_address(
        program_id,
        &author_pubkey,
        &settings_pubkey,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
    );

    let data = TokenProxyInstruction::WithdrawRequest {
        event_timestamp,
        event_transaction_lt,
        sender_address: EverAddress::with_standart(0, sender_address),
        event_configuration,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
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
    withdrawal_pubkey: String,
) -> JsValue {
    let program_id = &id();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).unwrap();
    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).unwrap();

    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).unwrap();

    let mint_pubkey = get_associated_mint_address(program_id, &name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::ApproveWithdrawEver
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
    withdrawal_pubkey: String,
) -> JsValue {
    let program_id = &id();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).unwrap();
    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).unwrap();

    let data = TokenProxyInstruction::ApproveWithdrawSol
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

#[wasm_bindgen(js_name = "withdrawalEver")]
pub fn withdrawal_ever_ix(
    to_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
) -> JsValue {
    let program_id = &id();

    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).unwrap();

    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).unwrap();

    let mint_pubkey = get_associated_mint_address(program_id, &name);
    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawEver
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
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

#[wasm_bindgen(js_name = "withdrawalSol")]
pub fn withdrawal_sol_ix(
    to_pubkey: String,
    name: String,
    withdrawal_pubkey: String,
) -> JsValue {
    let program_id = &id();
    let to_pubkey = Pubkey::from_str(to_pubkey.as_str()).unwrap();

    let vault_pubkey = get_associated_vault_address(program_id, &name);
    let settings_pubkey = get_associated_settings_address(program_id, &name);
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).unwrap();

    let mint_pubkey = get_associated_mint_address(program_id, &name);

    let recipient_pubkey =
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);

    let data = TokenProxyInstruction::WithdrawSol
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return JsValue::from_serde(&ix).unwrap();
}

#[wasm_bindgen(js_name = "voteForWithdrawRequest")]
pub fn vote_for_withdraw_request_ix(
    authority_pubkey: String,
    withdrawal_pubkey: String,
    round_number: u32,
) -> JsValue {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).unwrap();
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).unwrap();
    let relay_round_pubkey = get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = TokenProxyInstruction::VoteForWithdrawRequest {
        vote: Vote::Confirm,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
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
        account_kind: settings.account_kind,
        name: settings.name,
        kind: settings.kind,
        admin: settings.admin,
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
    let withdrawal = crate::WithdrawalToken::unpack(&data).unwrap();

    let w = Withdrawal {
        is_initialized: withdrawal.is_initialized,
        account_kind: withdrawal.account_kind,
        round_number: withdrawal.round_number,
        required_votes: withdrawal.required_votes,
        pda: withdrawal.pda,
        event: withdrawal.event,
        meta: withdrawal.meta,
        signers: withdrawal.signers,
    };

    return JsValue::from_serde(&w).unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub name: String,
    pub kind: TokenKind,
    pub admin: Pubkey,
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
    pub account_kind: AccountKind,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: WithdrawalTokenEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}
