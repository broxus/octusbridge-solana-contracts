use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

use bridge_derive::BridgePack;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use bridge_utils::state::*;
use bridge_utils::types::*;

use bridge_utils::helper::get_associated_relay_round_address;

use crate::*;

#[wasm_bindgen(js_name = "getMintAddress")]
pub fn get_mint_address_request(token: String) -> Result<JsValue, JsValue> {
    let token = EverAddress::from_str(&token).handle_error()?;
    let mint_pubkey = get_mint_address(&token);
    return serde_wasm_bindgen::to_value(&mint_pubkey).handle_error();
}

#[wasm_bindgen(js_name = "getTokenSettingsAddress")]
pub fn get_token_settings_request(token: String) -> Result<JsValue, JsValue> {
    let token = EverAddress::from_str(&token).handle_error()?;
    let token_settings_pubkey = get_token_settings_ever_address(&token);
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
    let settings_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let guardian = Pubkey::from_str(guardian.as_str()).handle_error()?;
    let withdrawal_manager = Pubkey::from_str(withdrawal_manager.as_str()).handle_error()?;
    let manager = Pubkey::from_str(manager.as_str()).handle_error()?;
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let initializer_pubkey = Pubkey::from_str(initializer_pubkey.as_str()).handle_error()?;
    let multivault_pubkey = get_multivault_address();

    let data = TokenProxyInstruction::Initialize {
        guardian,
        manager,
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
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let recipient = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let token = EverAddress::from_str(&token_address).handle_error()?;

    let amount = u128::from_str(&amount).handle_error()?;

    let token_settings_pubkey = get_token_settings_ever_address(&token);

    let relay_round_pubkey = get_associated_relay_round_address(&round_loader::id(), round_number);

    let withdrawal_pubkey = get_withdrawal_ever_address(
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
    );

    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let data = TokenProxyInstruction::WithdrawMultiTokenEverRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        token,
        name,
        symbol,
        decimals,
        recipient,
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
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let author_pubkey = Pubkey::from_str(author_pubkey.as_str()).handle_error()?;
    let mint = Pubkey::from_str(mint_pubkey.as_str()).handle_error()?;
    let recipient = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let token_settings_pubkey = get_token_settings_sol_address(&mint);

    let amount = u128::from_str(&amount).handle_error()?;

    let relay_round_pubkey = get_associated_relay_round_address(&round_loader::id(), round_number);

    let withdrawal_pubkey = get_withdrawal_sol_address(
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        mint,
        recipient,
        amount,
    );

    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());

    let data = TokenProxyInstruction::WithdrawMultiTokenSolRequest {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        recipient,
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "withdrawalMultiTokenEver")]
pub fn withdrawal_multi_token_ever_ix(
    funder_pubkey: String,
    withdrawal_pubkey: String,
    recipient_token_pubkey: String,
    token: String,
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;
    let token = EverAddress::from_str(&token).handle_error()?;

    let settings_pubkey = get_settings_address();
    let mint_pubkey = get_mint_address(&token);
    let token_settings_pubkey = get_token_settings_ever_address(&token);

    let data = TokenProxyInstruction::WithdrawMultiTokenEver
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
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

#[wasm_bindgen(js_name = "withdrawalMultiTokenSol")]
pub fn withdrawal_multi_token_sol_ix(
    funder_pubkey: String,
    withdrawal_pubkey: String,
    recipient_token_pubkey: String,
    mint: String,
) -> Result<JsValue, JsValue> {
    let funder_pubkey = Pubkey::from_str(funder_pubkey.as_str()).handle_error()?;
    let withdrawal_pubkey = Pubkey::from_str(withdrawal_pubkey.as_str()).handle_error()?;
    let mint = Pubkey::from_str(mint.as_str()).handle_error()?;
    let recipient_token_pubkey =
        Pubkey::from_str(recipient_token_pubkey.as_str()).handle_error()?;

    let settings_pubkey = get_settings_address();
    let vault_pubkey = get_vault_address(&mint);
    let token_settings_pubkey = get_token_settings_sol_address(&mint);

    let data = TokenProxyInstruction::WithdrawMultiTokenSol
        .try_to_vec()
        .handle_error()?;

    let ix = Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(funder_pubkey, true),
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "depositMultiTokenEver")]
pub fn deposit_multi_token_ever_ix(
    funder_pubkey: String,
    author_pubkey: String,
    author_token_pubkey: String,
    deposit_seed: String,
    recipient_address: String,
    token_address: String,
    amount: u64,
    sol_amount: u64,
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

    let mint_pubkey = get_mint_address(&token);
    let settings_pubkey = get_settings_address();
    let multivault_pubkey = get_multivault_address();
    let token_settings_pubkey = get_token_settings_ever_address(&token);
    let deposit_pubkey = get_deposit_address(deposit_seed);

    let payload = base64::decode(payload).handle_error()?;

    let data = TokenProxyInstruction::DepositMultiTokenEver {
        deposit_seed,
        recipient,
        amount,
        sol_amount,
        payload,
    }
    .try_to_vec()
    .handle_error()?;

    let ix = Instruction {
        program_id: id(),
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
    mint_pubkey: String,
    name: String,
    symbol: String,
    author_token_pubkey: String,
    deposit_seed: String,
    recipient_address: String,
    amount: u64,
    sol_amount: u64,
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

    let vault_pubkey = get_vault_address(&mint_pubkey);
    let settings_pubkey = get_settings_address();
    let multivault_pubkey = get_multivault_address();
    let token_settings_pubkey = get_token_settings_sol_address(&mint_pubkey);

    let deposit_pubkey = get_deposit_address(deposit_seed);

    let payload = base64::decode(payload).handle_error()?;

    let data = TokenProxyInstruction::DepositMultiTokenSol {
        deposit_seed,
        recipient,
        amount,
        name,
        symbol,
        sol_amount,
        payload,
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

/*#[wasm_bindgen(js_name = "changeGuardian")]
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableEmergencyByOwner")]
pub fn enable_emergency_by_owner_ix(authority_pubkey: String) -> Result<JsValue, JsValue> {
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "enableTokenEmergency")]
pub fn enable_token_emergency_ix(
    authority_pubkey: String,
    token_name: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&token_name);

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::EnableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
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
    token_name: String,
) -> Result<JsValue, JsValue> {
    let settings_pubkey = get_settings_address();
    let token_settings_pubkey = get_token_settings_address(&token_name);
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::EnableTokenEmergencyMode
        .try_to_vec()
        .expect("pack");

    let ix = Instruction {
        program_id: id(),
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
    token_name: String,
) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = get_token_settings_address(&token_name);
    let program_data_pubkey = get_programdata_address();

    let authority_pubkey = Pubkey::from_str(authority_pubkey.as_str()).handle_error()?;

    let data = TokenProxyInstruction::DisableTokenEmergencyMode
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

    return serde_wasm_bindgen::to_value(&ix).handle_error();
}

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let settings = Settings::unpack(&data).handle_error()?;

    let s = WasmSettings {
        emergency: settings.emergency,
        guardian: settings.guardian,
        withdrawal_manager: settings.withdrawal_manager,
    };

    return serde_wasm_bindgen::to_value(&s).handle_error();
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
        withdrawal_epoch: token_settings.withdrawal_epoch,
        emergency: token_settings.emergency,
    };

    return serde_wasm_bindgen::to_value(&s).handle_error();
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

    return serde_wasm_bindgen::to_value(&w).handle_error();
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

    return serde_wasm_bindgen::to_value(&d).handle_error();
}

#[wasm_bindgen(js_name = "getProposalAddress")]
pub fn get_proposal_address(
    token_settings: String,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: String,
    sender_address: String,
    recipient_address: String,
    amount: String,
) -> Result<JsValue, JsValue> {
    let token_settings_pubkey = Pubkey::from_str(token_settings.as_str()).handle_error()?;
    let recipient_address = Pubkey::from_str(recipient_address.as_str()).handle_error()?;
    let event_configuration = Pubkey::from_str(event_configuration.as_str()).handle_error()?;
    let amount = u128::from_str(&amount).handle_error()?;

    let sender_address = EverAddress::from_str(&sender_address).handle_error()?;

    let withdrawal_pubkey = get_withdrawal_address(
        &token_settings_pubkey,
        round_number,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        sender_address,
        recipient_address,
        amount,
    );

    return serde_wasm_bindgen::to_value(&withdrawal_pubkey).handle_error();
}*/

#[wasm_bindgen(js_name = "unpackSettings")]
pub fn unpack_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let settings = Settings::unpack(&data).handle_error()?;

    let s = WasmSettings {
        emergency: settings.emergency,
        guardian: settings.guardian,
        withdrawal_manager: settings.withdrawal_manager,
    };

    return serde_wasm_bindgen::to_value(&s).handle_error();
}

#[wasm_bindgen(js_name = "unpackTokenSettings")]
pub fn unpack_token_settings(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let token_settings = TokenSettings::unpack(&data).handle_error()?;

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
        fee_info: token_settings.fee_info,
    };

    return serde_wasm_bindgen::to_value(&s).handle_error();
}

#[wasm_bindgen(js_name = "unpackWithdrawalMultiTokenEver")]
pub fn unpack_withdrawal_multitoken_ever(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let withdrawal = WithdrawalMultiTokenEver::unpack(&data).handle_error()?;

    let w = WasmWithdrawalMultiTokenEver {
        is_initialized: withdrawal.is_initialized,
        account_kind: withdrawal.account_kind,
        is_executed: withdrawal.is_executed,
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
    let withdrawal = WithdrawalMultiTokenSol::unpack(&data).handle_error()?;

    let w = WasmWithdrawalMultiTokenSol {
        is_initialized: withdrawal.is_initialized,
        account_kind: withdrawal.account_kind,
        is_executed: withdrawal.is_executed,
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
    let deposit = DepositMultiTokenEver::unpack(&data).handle_error()?;

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
    let deposit = DepositMultiTokenSol::unpack(&data).handle_error()?;

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
    pub fee_info: FeeInfo,
}

#[derive(Serialize, Deserialize)]
pub struct WasmWithdrawalMultiTokenEver {
    pub is_initialized: bool,
    pub is_executed: bool,
    pub account_kind: AccountKind,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: WithdrawalMultiTokenEverEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmWithdrawalMultiTokenSol {
    pub is_initialized: bool,
    pub is_executed: bool,
    pub account_kind: AccountKind,
    pub author: Pubkey,
    pub round_number: u32,
    pub required_votes: u32,
    pub pda: PDA,
    pub event: WithdrawalMultiTokenSolEventWithLen,
    pub meta: WithdrawalTokenMetaWithLen,
    pub signers: Vec<Vote>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositMultiTokenEver {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: DepositMultiTokenEverEventWithLen,
    pub meta: WasmDepositTokenMeta,
}

#[derive(Serialize, Deserialize)]
pub struct WasmDepositMultiTokenSol {
    pub is_initialized: bool,
    pub account_kind: AccountKind,
    pub event: DepositMultiTokenSolEventWithLen,
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

impl From<TokenKind> for WasmTokenKind {
    fn from(t: TokenKind) -> Self {
        match t {
            TokenKind::Ever {
                mint,
                token,
                decimals,
            } => WasmTokenKind::Ever {
                mint: mint.to_string(),
                token: token.to_string(),
                decimals,
            },
            TokenKind::Solana { mint, vault } => WasmTokenKind::Solana {
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
