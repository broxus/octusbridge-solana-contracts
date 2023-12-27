use std::str::FromStr;

use base64::engine::general_purpose;
use base64::Engine;
use borsh::BorshSerialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use bridge_utils::types::*;

use crate::*;

#[wasm_bindgen(js_name = "depositNativeSol")]
pub fn deposit_native_sol_ix(
    funder_pubkey: String,
    author_pubkey: String,
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

    let data = NativeProxyInstruction::Deposit {
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
            AccountMeta::new_readonly(token_proxy::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data,
    };

    return serde_wasm_bindgen::to_value(&ix).handle_error();
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
