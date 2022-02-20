use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::{Settings, TokenKind, TokenProxyInstruction};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = TokenProxyInstruction::try_from_slice(instruction_data).unwrap();

        match instruction {
            TokenProxyInstruction::Initialize {
                name,
                kind,
                withdrawal_limit,
                deposit_limit,
                decimals,
                admin,
                token,
            } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(
                    program_id,
                    accounts,
                    name,
                    kind,
                    withdrawal_limit,
                    deposit_limit,
                    decimals,
                    admin,
                    token,
                )?;
            }
        };

        Ok(())
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        kind: TokenKind,
        withdrawal_limit: u64,
        deposit_limit: u64,
        decimals: u8,
        admin: Pubkey,
        token: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_programdata_account(program_id, programdata_account_info.key)?;

        bridge_utils::validate_creator_account(creator_account_info.key, programdata_account_info)?;

        // Create Settings Account
        let settings_nonce = bridge_utils::validate_tp_settings_account(
            program_id,
            settings_account_info.key,
            &name,
        )?;
        let settings_account_signer_seeds: &[&[_]] =
            &[b"settings", &name.as_bytes(), &[settings_nonce]];

        bridge_utils::fund_account(
            settings_account_info,
            funder_account_info,
            system_program_info,
            Settings::LEN,
        )?;

        bridge_utils::create_account(
            program_id,
            settings_account_info,
            system_program_info,
            settings_account_signer_seeds,
            Settings::LEN,
        )?;

        let settings_account_data = Settings {
            is_initialized: true,
            name: name.clone(),
            kind,
            withdrawal_limit,
            deposit_limit,
            decimals,
            admin,
            token,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Suppose that token root or token vault account was created before and authority was transferred to
        // token proxy so there is no need to do anything here with them

        Ok(())
    }
}
