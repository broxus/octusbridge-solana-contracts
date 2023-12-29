use borsh::BorshSerialize;
use bridge_utils::types::{EverAddress, UInt256};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use crate::*;

pub fn deposit_ix(
    funder_pubkey: Pubkey,
    author_pubkey: Pubkey,
    deposit_seed: u128,
    amount: u64,
    recipient: EverAddress,
    value: u64,
    expected_evers: UInt256,
    payload: Vec<u8>,
) -> Instruction {
    let mint_pubkey = spl_token::native_mint::id();

    let author_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&author_pubkey, &mint_pubkey);

    let vault_pubkey = token_proxy::get_vault_address(&mint_pubkey);
    let settings_pubkey = token_proxy::get_settings_address();
    let multivault_pubkey = token_proxy::get_multivault_address();
    let token_settings_pubkey = token_proxy::get_token_settings_sol_address(&mint_pubkey);

    let deposit_pubkey = token_proxy::get_deposit_address(deposit_seed);

    let data = NativeProxyInstruction::Deposit {
        deposit_seed,
        amount,
        recipient,
        value,
        expected_evers,
        payload,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
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
    }
}
