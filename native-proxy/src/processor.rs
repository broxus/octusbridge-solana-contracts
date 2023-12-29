use borsh::BorshDeserialize;

use bridge_utils::types::{EverAddress, UInt256};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_instruction};

use crate::*;

const WSOL_SYMBOL: &str = "wSOL";
const WSOL_NAME: &str = "Wrapped SOL";

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = NativeProxyInstruction::try_from_slice(instruction_data)?;

        match instruction {
            NativeProxyInstruction::Deposit {
                deposit_seed,
                amount,
                recipient,
                value,
                expected_evers,
                payload,
            } => {
                msg!("Instruction: Wrapping SOL");
                Self::process_deposit(
                    program_id,
                    accounts,
                    deposit_seed,
                    amount,
                    recipient,
                    value,
                    expected_evers,
                    payload,
                )?;
            }
        };

        Ok(())
    }

    fn process_deposit(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        amount: u64,
        recipient: EverAddress,
        value: u64,
        expected_evers: UInt256,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let creator_token_account_info = next_account_info(account_info_iter)?;
        let _vault_account_info = next_account_info(account_info_iter)?;
        let _deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;

        if *mint_account_info.key != spl_token::native_mint::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let token_pubkey = spl_associated_token_account::get_associated_token_address(
            creator_account_info.key,
            &spl_token::native_mint::id(),
        );

        if token_pubkey != *creator_token_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        if creator_token_account_info.lamports() == 0 {
            // Create Token Account
            invoke(
                &spl_associated_token_account::instruction::create_associated_token_account(
                    funder_account_info.key,
                    creator_account_info.key,
                    &spl_token::native_mint::id(),
                    &spl_token::id(),
                ),
                accounts,
            )?;
        }

        invoke(
            &system_instruction::transfer(creator_account_info.key, &token_pubkey, amount),
            accounts,
        )?;

        invoke(
            &spl_token::instruction::sync_native(&spl_token::id(), &token_pubkey)?,
            accounts,
        )?;

        let name = WSOL_NAME.to_string();
        let symbol = WSOL_SYMBOL.to_string();

        invoke(
            &token_proxy::deposit_multi_token_sol_ix(
                *funder_account_info.key,
                *creator_account_info.key,
                *creator_token_account_info.key,
                spl_token::native_mint::id(),
                deposit_seed,
                name,
                symbol,
                amount,
                recipient,
                value,
                expected_evers,
                payload,
            ),
            accounts,
        )?;

        Ok(())
    }
}
