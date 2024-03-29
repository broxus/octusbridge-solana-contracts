use std::str::FromStr;

use base64::engine::general_purpose;
use base64::Engine;
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

#[wasm_bindgen(js_name = "getMintAddress")]
pub fn get_mint_address_request(token: String) -> Result<JsValue, JsValue> {
    let token = EverAddress::from_str(&token).handle_error()?;
    let mint_pubkey = token_proxy::get_mint_address(&token);
    return serde_wasm_bindgen::to_value(&mint_pubkey).handle_error();
}

#[wasm_bindgen(js_name = "getTokenSettingsAddress")]
pub fn get_token_settings_request(token: String, is_sol: bool) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = if is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };
    return serde_wasm_bindgen::to_value(&token_settings_pubkey).handle_error();
}

#[wasm_bindgen(js_name = "initializeSettings")]
pub fn initialize_settings_ix(
    funder_pubkey: String,
    initializer_pubkey: String,
    guardian: String,
    withdrawal_manager: String,
    manager: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let program_data_pubkey = token_proxy::get_programdata_address();

    let guardian = Pubkey::from_str(guardian.as_str()).handle_error()?;
    let withdrawal_manager = Pubkey::from_str(withdrawal_manager.as_str()).handle_error()?;
    let manager = Pubkey::from_str(manager.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).handle_error()?;
    let multivault_pubkey = token_proxy::get_multivault_address();

    let data = token_proxy::TokenProxyInstruction::Initialize {
        guardian,
        manager,
        withdrawal_manager,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(initializer_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new(multivault_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalMultiTokenEverRequest")]
pub fn withdrawal_multi_token_ever_request_ix(
    funder_pubkey: String,
    author_pubkey: String,
    name: String,
    symbol: String,
    decimals: u8,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    token_address: String,
    recipient_address: String,
    amount: String,
    round_number: u32,
    payload: String,
    attached_amount: u64,
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let token = EverAddress::from_str(&token_address).handle_error()?;
    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let amount = u128::from_str(&amount).handle_error()?;

    let relay_round_pubkey =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let withdrawal_pubkey = token_proxy::get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name.clone(),
        symbol.clone(),
        decimals,
        recipient,
        amount,
        payload.clone(),
    );

    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let mut accounts = vec![
        AccountMeta::new(funder_pubkey, true),
        AccountMeta::new(author_pubkey, true),
        AccountMeta::new(withdrawal_pubkey, false),
        AccountMeta::new_readonly(rl_settings_pubkey, false),
        AccountMeta::new_readonly(relay_round_pubkey, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    if !payload.is_empty() {
        let mint_pubkey = token_proxy::get_mint_address(&token);
        let proxy_pubkey = token_proxy::get_proxy_address(&mint_pubkey, &recipient);

        accounts.push(AccountMeta::new(proxy_pubkey, false));
        accounts.push(AccountMeta::new(mint_pubkey, false));
        accounts.push(AccountMeta::new(spl_token::id(), false));
    }

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenEverRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        token,
        name,
        symbol,
        decimals,
        recipient,
        amount,
        payload,
        attached_amount,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts,
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalMultiTokenSolRequest")]
pub fn withdrawal_multi_token_sol_request_ix(
    funder_pubkey: String,
    author_pubkey: String,
    mint_pubkey: String,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    recipient_address: String,
    amount: String,
    round_number: u32,
    payload: String,
    attached_amount: u64,
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let mint = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let recipient = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint);
    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let amount = u128::from_str(&amount).handle_error()?;

    let relay_round_pubkey =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let withdrawal_pubkey = token_proxy::get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint,
        recipient,
        amount,
        payload.clone(),
    );

    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let mut accounts = vec![
        AccountMeta::new(funder_pubkey, true),
        AccountMeta::new(author_pubkey, true),
        AccountMeta::new(withdrawal_pubkey, false),
        AccountMeta::new_readonly(token_settings_pubkey, false),
        AccountMeta::new_readonly(rl_settings_pubkey, false),
        AccountMeta::new_readonly(relay_round_pubkey, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    if !payload.is_empty() {
        let proxy_pubkey = token_proxy::get_proxy_address(&mint, &recipient);

        accounts.push(AccountMeta::new(proxy_pubkey, false));
        accounts.push(AccountMeta::new(mint, false));
        accounts.push(AccountMeta::new(spl_token::id(), false));
    }

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenSolRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        recipient,
        amount,
        payload,
        attached_amount,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts,
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalMultiTokenEver")]
pub fn withdrawal_multi_token_ever_ix(
    withdrawal_pubkey: String,
    recipient_token_pubkey: String,
    token: String,
) -> Result<JsValue, JsValue> {
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;
    let token = EverAddress::from_str(&token).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let mint_pubkey = token_proxy::get_mint_address(&token);
    let token_settings_pubkey = token_proxy::get_token_settings_ever_address(&token);

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenEver
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "createMultiTokenEver")]
pub fn create_ever_token_ix(
    funder_pubkey: String,
    withdrawal_pubkey: String,
    recipient_pubkey: String,
    token: String,
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_pubkey = Pubkey::from_str(recipient_pubkey.as_str()).handle_error()?;
    let token = EverAddress::from_str(&token).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let mint_pubkey = token_proxy::get_mint_address(&token);
    let token_settings_pubkey = token_proxy::get_token_settings_ever_address(&token);

    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&recipient_pubkey, &mint_pubkey);

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenEver
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            // If token settings account is not created
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new_readonly(recipient_pubkey, false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalMultiTokenSol")]
pub fn withdrawal_multi_token_sol_ix(
    withdrawal_pubkey: String,
    recipient_token_pubkey: String,
    mint: String,
) -> Result<JsValue, JsValue> {
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let mint = Pubkey::from_str(mint.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let vault_pubkey = token_proxy::get_vault_address(&mint);
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint);

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenSol
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "depositNativeSol")]
pub fn deposit_native_sol_ix(
    funder_pubkey: String,
    author_pubkey: String,
    deposit_seed: String,
    amount: u64,
    recipient_address: String,
    value: u64,
    expected_evers: u64,
    payload: String,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient = EverAddress::from_str(&recipient_address).handle_error()?;

    let expected_evers = UInt256::from_be_bytes(expected_evers.to_be_bytes().as_slice());

    let mint_pubkey = spl_token::native_mint::id();

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&author_pubkey, &mint_pubkey);

    let vault_pubkey = token_proxy::get_vault_address(&mint_pubkey);
    let settings_pubkey = token_proxy::get_settings_address();
    let multivault_pubkey = token_proxy::get_multivault_address();
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let deposit_pubkey = token_proxy::get_deposit_address(deposit_seed);

    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let data = native_proxy::NativeProxyInstruction::Deposit {
        deposit_seed,
        amount,
        recipient,
        value,
        expected_evers,
        payload,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: native_proxy::id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(multivault_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(token_proxy::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "depositMultiTokenEver")]
pub fn deposit_multi_token_ever_ix(
    funder_pubkey: String,
    author_pubkey: String,
    author_token_pubkey: String,
    token_address: String,
    deposit_seed: String,
    amount: u64,
    recipient_address: String,
    value: u64,
    expected_evers: u64,
    payload: String,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient = EverAddress::from_str(&recipient_address).handle_error()?;
    let token = EverAddress::from_str(&token_address).handle_error()?;
    let author_token_pubkey = Pubkey::from_str(author_token_pubkey.as_str()).handle_error()?;

    let expected_evers = UInt256::from_be_bytes(expected_evers.to_be_bytes().as_slice());

    let mint_pubkey = token_proxy::get_mint_address(&token);
    let settings_pubkey = token_proxy::get_settings_address();
    let multivault_pubkey = token_proxy::get_multivault_address();
    let token_settings_pubkey = token_proxy::get_token_settings_ever_address(&token);
    let deposit_pubkey = token_proxy::get_deposit_address(deposit_seed);

    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::DepositMultiTokenEver {
        deposit_seed,
        amount,
        recipient,
        value,
        expected_evers,
        payload,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(multivault_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "depositMultiTokenSol")]
pub fn deposit_multi_token_sol_ix(
    funder_pubkey: String,
    author_pubkey: String,
    author_token_pubkey: String,
    mint_pubkey: String,
    deposit_seed: String,
    name: String,
    symbol: String,
    amount: u64,
    recipient_address: String,
    value: u64,
    expected_evers: u64,
    payload: String,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let author_token_pubkey = Pubkey::from_str(author_token_pubkey.as_str()).handle_error()?;
    let recipient = EverAddress::from_str(&recipient_address).handle_error()?;

    let expected_evers = UInt256::from_be_bytes(expected_evers.to_be_bytes().as_slice());

    let vault_pubkey = token_proxy::get_vault_address(&mint_pubkey);
    let settings_pubkey = token_proxy::get_settings_address();
    let multivault_pubkey = token_proxy::get_multivault_address();
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let deposit_pubkey = token_proxy::get_deposit_address(deposit_seed);

    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::DepositMultiTokenSol {
        deposit_seed,
        name,
        symbol,
        amount,
        recipient,
        value,
        expected_evers,
        payload,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(multivault_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "executePayloadSol")]
pub fn execute_payload_sol_ix(
    withdrawal_pubkey: String,
    recipient_address: String,
    mint_address: String,
    recipient_token_pubkey: String,
) -> Result<JsValue, JsValue> {
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let mint_address = Pubkey::from_str(mint_address.as_str()).handle_error()?;
    let proxy_address = token_proxy::get_proxy_address(&mint_address, &recipient_address);
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::ExecutePayloadSol
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(proxy_address, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "executePayloadEver")]
pub fn execute_payload_ever_ix(
    withdrawal_pubkey: String,
    recipient_address: String,
    mint_address: String,
    recipient_token_pubkey: String,
) -> Result<JsValue, JsValue> {
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let mint_address = Pubkey::from_str(mint_address.as_str()).handle_error()?;
    let proxy_address = token_proxy::get_proxy_address(&mint_address, &recipient_address);
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::ExecutePayloadEver
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(proxy_address, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "voteForWithdrawRequest")]
pub fn vote_for_withdraw_request_ix(
    authority_pubkey: String,
    withdrawal_pubkey: String,
    round_number: u32,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let relay_round_pubkey =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = token_proxy::TokenProxyInstruction::VoteForWithdrawRequest {
        vote: Vote::Confirm,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeGuardian")]
pub fn change_guardian_ix(
    authority_pubkey: String,
    new_guardian: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let new_guardian = Pubkey::from_str(new_guardian.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::ChangeGuardian { new_guardian }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeWithdrawalManager")]
pub fn change_withdrawal_manager_ix(
    authority_pubkey: String,
    new_withdrawal_manager: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let new_withdrawal_manager =
        Pubkey::from_str(new_withdrawal_manager.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::ChangeWithdrawalManager {
        new_withdrawal_manager,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeDepositLimit")]
pub fn change_deposit_limit_ix(
    authority_pubkey: String,
    token: String,
    token_is_sol: bool,
    new_deposit_limit: u64,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let token_settings_pubkey = if token_is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };
    let program_data_pubkey = token_proxy::get_programdata_address();

    let data = token_proxy::TokenProxyInstruction::ChangeDepositLimit { new_deposit_limit }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeWithdrawalLimits")]
pub fn change_withdrawal_limits_ix(
    authority_pubkey: String,
    token: String,
    token_is_sol: bool,
    new_withdrawal_limit: Option<u64>,
    new_withdrawal_daily_limit: Option<u64>,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();

    let token_settings_pubkey = if token_is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::ChangeWithdrawalLimits {
        new_withdrawal_limit,
        new_withdrawal_daily_limit,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableEmergency")]
pub fn enable_emergency_ix(authority_pubkey: String) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::EnableEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableEmergencyByOwner")]
pub fn enable_emergency_by_owner_ix(authority_pubkey: String) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::EnableEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "disableEmergency")]
pub fn disable_emergency_ix(authority_pubkey: String) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::DisableEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableTokenEmergency")]
pub fn enable_token_emergency_ix(
    authority_pubkey: String,
    token: String,
    token_is_sol: bool,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let token_settings_pubkey = if token_is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::EnableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableTokenEmergencyByOwner")]
pub fn enable_token_emergency_by_owner_ix(
    authority_pubkey: String,
    token: String,
    token_is_sol: bool,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = token_proxy::get_settings_address();
    let token_settings_pubkey = if token_is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::EnableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "disableTokenEmergency")]
pub fn disable_token_emergency_ix(
    authority_pubkey: String,
    token: String,
    token_is_sol: bool,
) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = if token_is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };
    let program_data_pubkey = token_proxy::get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::DisableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalProxy")]
pub fn withdrawal_proxy_ix(
    recipient_pubkey: String,
    recipient_token_pubkey: String,
    mint_pubkey: String,
    amount: u64,
) -> Result<JsValue, JsValue> {
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let recipient_pubkey = Pubkey::from_str(recipient_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;

    let proxy_pubkey = token_proxy::get_proxy_address(&mint_pubkey, &recipient_pubkey);

    let data = token_proxy::TokenProxyInstruction::WithdrawProxy { amount }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(recipient_pubkey, true),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(proxy_pubkey, false),
            AccountMeta::new_readonly(mint_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "closeDeposit")]
pub fn close_deposit(author_address: String, deposit_address: String) -> Result<JsValue, JsValue> {
    let author_address = Pubkey::from_str(author_address.as_str()).handle_error()?;
    let deposit_address = Pubkey::from_str(deposit_address.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::CloseDeposit
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(author_address, true),
            AccountMeta::new(deposit_address, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "closeWithdrawal")]
pub fn close_withdrawal(
    withdrawal_address: String,
    withdrawal_author_address: String,
) -> Result<JsValue, JsValue> {
    let withdrawal_address = Pubkey::from_str(withdrawal_address.as_str()).handle_error()?;
    let withdrawal_author_address =
        Pubkey::from_str(withdrawal_author_address.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::CloseWithdrawal
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(withdrawal_address, false),
            AccountMeta::new(withdrawal_author_address, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "updateFee")]
pub fn update_fee(
    authority_pubkey: String,
    token: String,
    token_is_sol: bool,
    fee_type: String,
    multiplier: u64,
    divisor: u64,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let token_settings_pubkey = if token_is_sol {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    } else {
        let token = EverAddress::from_str(&token).handle_error()?;
        token_proxy::get_token_settings_ever_address(&token)
    };

    let settings_pubkey = token_proxy::get_settings_address();

    let data = token_proxy::TokenProxyInstruction::UpdateFee {
        fee_type: token_proxy::FeeType::from_str(&fee_type).handle_error()?,
        multiplier,
        divisor,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "updateTokenName")]
pub fn update_token_name(
    authority_pubkey: String,
    token: String,
    symbol: String,
    name: String,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let token_settings_pubkey = {
        let mint = Pubkey::from_str(token.as_str()).handle_error()?;
        token_proxy::get_token_settings_sol_address(&mint)
    };

    let settings_pubkey = token_proxy::get_settings_address();

    let data = token_proxy::TokenProxyInstruction::UpdateTokenName { symbol, name }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "approveWithdrawalEver")]
pub fn approve_withdrawal_ever(
    authority_pubkey: String,
    withdrawal_pubkey: String,
    recipient_token_pubkey: String,
    mint_pubkey: String,
    token: String,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let token = EverAddress::from_str(&token).handle_error()?;
    let token_settings_pubkey = token_proxy::get_token_settings_ever_address(&token);

    let data = token_proxy::TokenProxyInstruction::ApproveWithdrawEver
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "approveWithdrawalSol")]
pub fn approve_withdrawal_sol(
    authority_pubkey: String,
    withdrawal_pubkey: String,
    recipient_token_pubkey: String,
    mint_pubkey: String,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let vault_pubkey = token_proxy::get_vault_address(&mint_pubkey);
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let data = token_proxy::TokenProxyInstruction::ApproveWithdrawSol
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "cancelWithdrawalSol")]
pub fn cancel_withdrawal_sol(
    funder_pubkey: String,
    author_pubkey: String,
    withdrawal_pubkey: String,
    mint_pubkey: String,
    deposit_seed: String,
    recipient: String,
    value: u64,
    expected_evers: u64,
    payload: String,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient = EverAddress::from_str(&recipient).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let multivault_pubkey = token_proxy::get_multivault_address();
    let deposit_pubkey = token_proxy::get_deposit_address(deposit_seed);
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let expected_evers = UInt256::from_be_bytes(expected_evers.to_be_bytes().as_slice());

    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::CancelWithdrawSol {
        deposit_seed,
        recipient,
        value,
        expected_evers,
        payload,
    }
    .try_to_vec()
    .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(multivault_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Deserialize)]
pub struct FillWithdrawals {
    pub withdrawal_pubkey: String,
    pub to_pubkey: String,
}

#[wasm_bindgen(js_name = "fillWithdrawalSol")]
pub fn fill_withdrawal_sol(
    funder_pubkey: String,
    author_pubkey: String,
    mint_pubkey: String,
    deposit_seed: String,
    recipient: String,
    amount: u64,
    withdrawal_pubkeys: Vec<JsValue>,
    vault_pubkey: Option<String>,
    value: u64,
    expected_evers: u64,
    payload: String,
) -> Result<JsValue, JsValue> {
    let deposit_seed = uuid::Uuid::from_str(&deposit_seed)
        .handle_error()?
        .as_u128();

    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient = EverAddress::from_str(&recipient).handle_error()?;

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&author_pubkey, &mint_pubkey);

    let settings_pubkey = token_proxy::get_settings_address();
    let multivault_pubkey = token_proxy::get_multivault_address();
    let deposit_pubkey = token_proxy::get_deposit_address(deposit_seed);
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let expected_evers = UInt256::from_be_bytes(expected_evers.to_be_bytes().as_slice());
    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::FillWithdrawSol {
        deposit_seed,
        recipient,
        amount,
        value,
        expected_evers,
        payload,
    }
    .try_to_vec()
    .expect("pack");

    let mut ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(author_token_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(token_settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(multivault_pubkey, false),
        ],
        data,
    };

    for fill in withdrawal_pubkeys {
        let fill: FillWithdrawals = serde_wasm_bindgen::from_value(fill).handle_error()?;
        let withdrawal_pubkey = Pubkey::from_str(fill.withdrawal_pubkey.as_str()).handle_error()?;
        let to_pubkey = Pubkey::from_str(fill.to_pubkey.as_str()).handle_error()?;
        let recipient_token_pubkey =
            spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey);
        ix.accounts.push(AccountMeta::new(withdrawal_pubkey, false));
        ix.accounts
            .push(AccountMeta::new(recipient_token_pubkey, false));
    }

    if let Some(vault_pubkey) = vault_pubkey {
        let vault_pubkey = Pubkey::from_str(vault_pubkey.as_str()).handle_error()?;
        ix.accounts.push(AccountMeta::new(vault_pubkey, false));
    }

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "changeBountyForWithdrawalSol")]
pub fn change_bounty_for_withdrawal_sol_ix(
    author_pubkey: String,
    withdrawal_pubkey: String,
    bounty: u64,
) -> Result<JsValue, JsValue> {
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;

    let data = token_proxy::TokenProxyInstruction::ChangeBountyForWithdrawSol { bounty }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalMultiVault")]
pub fn withdrawal_multi_vault_ix(
    authority_pubkey: String,
    recipient_pubkey: String,
    amount: u64,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let recipient_pubkey = Pubkey::from_str(recipient_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let multi_vault_pubkey = token_proxy::get_multivault_address();

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiVault { amount }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new(multi_vault_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalEverFee")]
pub fn withdrawal_ever_fee_ix(
    authority_pubkey: String,
    mint_pubkey: String,
    recipient_token_pubkey: String,
    token: String,
    amount: u64,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();

    let token = EverAddress::from_str(&token).handle_error()?;
    let token_settings_pubkey = token_proxy::get_token_settings_ever_address(&token);

    let data = token_proxy::TokenProxyInstruction::WithdrawEverFee { amount }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalSolFee")]
pub fn withdrawal_sol_fee_ix(
    authority_pubkey: String,
    recipient_token_pubkey: String,
    mint_pubkey: String,
    amount: u64,
) -> Result<JsValue, JsValue> {
    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;
    let mint_pubkey = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;

    let settings_pubkey = token_proxy::get_settings_address();
    let vault_pubkey = token_proxy::get_vault_address(&mint_pubkey);
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let data = token_proxy::TokenProxyInstruction::WithdrawSolFee { amount }
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: token_proxy::id(),
        accounts: vec![
            AccountMeta::new(authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(token_settings_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let settings = token_proxy::Settings::unpack(&data).handle_error()?;

    let s = WasmSettings {
        emergency: settings.emergency,
        guardian: settings.guardian,
        withdrawal_manager: settings.withdrawal_manager,
        manager: settings.manager,
    };

    return serde_wasm_bindgen::to_value(&s).handle_error();
}

#[wasm_bindgen(js_name = "getProposalSolAddress")]
pub fn get_proposal_sol_address(
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    mint_address: String,
    recipient_address: String,
    amount: String,
    payload: String,
) -> Result<JsValue, JsValue> {
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let amount = u128::from_str(&amount).handle_error()?;

    let mint_address = Pubkey::from_str(mint_address.as_str()).handle_error()?;

    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let withdrawal_pubkey = token_proxy::get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint_address,
        recipient_address,
        amount,
        payload,
    );

    return serde_wasm_bindgen::to_value(&withdrawal_pubkey).handle_error();
}

#[wasm_bindgen(js_name = "getProxyAddress")]
pub fn get_proxy_address_payload(
    mint_address: String,
    recipient_address: String,
) -> Result<JsValue, JsValue> {
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let mint_address = Pubkey::from_str(mint_address.as_str()).handle_error()?;
    let proxy_address = token_proxy::get_proxy_address(&mint_address, &recipient_address);
    return serde_wasm_bindgen::to_value(&proxy_address).handle_error();
}

#[wasm_bindgen(js_name = "getProposalEverAddress")]
pub fn get_proposal_ever_address(
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    token_address: String,
    recipient_address: String,
    amount: String,
    name: String,
    symbol: String,
    decimals: u8,
    payload: String,
) -> Result<JsValue, JsValue> {
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let amount = u128::from_str(&amount).handle_error()?;

    let token = EverAddress::from_str(&token_address).handle_error()?;

    let payload = general_purpose::STANDARD.decode(payload).handle_error()?;

    let withdrawal_pubkey = token_proxy::get_withdrawal_ever_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        token,
        name,
        symbol,
        decimals,
        recipient_address,
        amount,
        payload,
    );

    return serde_wasm_bindgen::to_value(&withdrawal_pubkey).handle_error();
}

#[wasm_bindgen(js_name = "unpackTokenSettings")]
pub fn unpack_token_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let token_settings = token_proxy::TokenSettings::unpack(&data).handle_error()?;

    let s = WasmTokenSettings {
        is_initialized: token_settings.is_initialized,
        account_kind: token_settings.account_kind,
        kind: token_settings.kind.into(),
        deposit_limit: token_settings.deposit_limit.to_string(),
        withdrawal_limit: token_settings.withdrawal_limit.to_string(),
        withdrawal_daily_limit: token_settings.withdrawal_daily_limit.to_string(),
        withdrawal_daily_amount: token_settings.withdrawal_daily_amount.to_string(),
        withdrawal_epoch: token_settings.withdrawal_epoch.to_string(),
        emergency: token_settings.emergency,
        name: token_settings.name,
        symbol: token_settings.symbol,
        fee_supply: token_settings.fee_supply,
        fee_deposit_info: token_settings.fee_deposit_info,
        fee_withdrawal_info: token_settings.fee_withdrawal_info,
    };

    return serde_wasm_bindgen::to_value(&s).handle_error();
}

#[wasm_bindgen(js_name = "unpackWithdrawalMultiTokenEver")]
pub fn unpack_withdrawal_multitoken_ever(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let withdrawal = token_proxy::WithdrawalMultiTokenEver::unpack(&data).handle_error()?;

    let w = WasmWithdrawalMultiTokenEver {
        is_initialized: withdrawal.is_initialized,
        account_kind: withdrawal.account_kind,
        author: withdrawal.author,
        round_number: withdrawal.round_number,
        required_votes: withdrawal.required_votes,
        pda: withdrawal.pda,
        event: withdrawal.event,
        meta: withdrawal.meta,
        signers: withdrawal.signers,
    };

    return serde_wasm_bindgen::to_value(&w).handle_error();
}

#[wasm_bindgen(js_name = "unpackWithdrawalMultiTokenSol")]
pub fn unpack_withdrawal_multitoken_sol(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let withdrawal = token_proxy::WithdrawalMultiTokenSol::unpack(&data).handle_error()?;

    let w = WasmWithdrawalMultiTokenSol {
        is_initialized: withdrawal.is_initialized,
        account_kind: withdrawal.account_kind,
        author: withdrawal.author,
        round_number: withdrawal.round_number,
        required_votes: withdrawal.required_votes,
        pda: withdrawal.pda,
        event: withdrawal.event,
        meta: withdrawal.meta,
        signers: withdrawal.signers,
    };

    return serde_wasm_bindgen::to_value(&w).handle_error();
}

#[wasm_bindgen(js_name = "unpackDepositEver")]
pub fn unpack_deposit_ever(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let deposit = token_proxy::DepositMultiTokenEver::unpack(&data).handle_error()?;

    let d = WasmDepositMultiTokenEver {
        is_initialized: deposit.is_initialized,
        account_kind: deposit.account_kind,
        event: deposit.event,
        meta: WasmDepositTokenMeta {
            seed: deposit.meta.data.seed.to_string(),
        },
    };

    return serde_wasm_bindgen::to_value(&d).handle_error();
}

#[wasm_bindgen(js_name = "unpackDepositSol")]
pub fn unpack_deposit_sol(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let deposit = token_proxy::DepositMultiTokenSol::unpack(&data).handle_error()?;

    let d = WasmDepositMultiTokenSol {
        is_initialized: deposit.is_initialized,
        account_kind: deposit.account_kind,
        event: deposit.event,
        meta: WasmDepositTokenMeta {
            seed: deposit.meta.data.seed.to_string(),
        },
    };

    return serde_wasm_bindgen::to_value(&d).handle_error();
}

#[derive(Serialize, Deserialize)]
pub struct WasmSettings {
    pub emergency: bool,
    pub guardian: Pubkey,
    pub withdrawal_manager: Pubkey,
    pub manager: Pubkey,
}

#[derive(Serialize, Deserialize)]
pub struct WasmTokenSettings {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub kind: WasmTokenKind,
    pub deposit_limit: String,
    pub withdrawal_limit: String,
    pub withdrawal_daily_limit: String,
    pub withdrawal_daily_amount: String,
    pub withdrawal_epoch: String,
    pub emergency: bool,
    pub name: String,
    pub symbol: String,
    pub fee_supply: u64,
    pub fee_deposit_info: token_proxy::FeeInfo,
    pub fee_withdrawal_info: token_proxy::FeeInfo,
}

#[derive(Serialize, Deserialize)]
pub struct WasmWithdrawalMultiTokenEver {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: token_proxy::WithdrawalMultiTokenEverEventWithLen,
    pub meta: token_proxy::WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmWithdrawalMultiTokenSol {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: token_proxy::WithdrawalMultiTokenSolEventWithLen,
    pub meta: token_proxy::WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositMultiTokenEver {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: token_proxy::DepositMultiTokenEverEventWithLen,
    pub meta: WasmDepositTokenMeta,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositMultiTokenSol {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: token_proxy::DepositMultiTokenSolEventWithLen,
    pub meta: WasmDepositTokenMeta,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositTokenMeta {
    pub seed: String,
}

#[derive(Serialize, Deserialize)]
pub enum WasmTokenKind {
    Ever {
        mint: String,
        token: String,
        decimals: u8,
    },
    Solana {
        mint: String,
        vault: String,
    },
}

impl From<token_proxy::TokenKind> for WasmTokenKind {
    fn from(t: token_proxy::TokenKind) -> Self {
        match t {
            token_proxy::TokenKind::Ever {
                mint,
                token,
                decimals,
            } => WasmTokenKind::Ever {
                mint: mint.to_string(),
                token: token.to_string(),
                decimals,
            },
            token_proxy::TokenKind::Solana { mint, vault } => WasmTokenKind::Solana {
                mint: mint.to_string(),
                vault: vault.to_string(),
            },
        }
    }
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
