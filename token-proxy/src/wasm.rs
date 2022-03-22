use std::str::FromStr;

use borsh::BorshSerialize;
use wasm_bindgen::prelude::*;

use solana_program::hash::Hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::{
    get_associated_mint_address, get_associated_settings_address,
    get_associated_withdrawal_address, id, TokenProxyInstruction,
};

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
