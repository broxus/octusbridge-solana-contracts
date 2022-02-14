use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::{
    Settings, TokenKind, TokenProxyError, TokenProxyInstruction,
};

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
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let program_account_info = next_account_info(account_info_iter)?;
        let program_buffer_account_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if program_account_info.key != program_id {
            return Err(ProgramError::InvalidAccountData);
        }

        validate_authority(
            authority_account_info,
            program_account_info,
            program_buffer_account_info,
        )?;

        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        // Create Settings account
        let settings_nonce = validate_settings_account(program_id, &name,settings_account_info.key)?;
        let settings_account_signer_seeds: &[&[_]] = &[b"settings", &name, &[settings_nonce]];

        create_account(
            program_id,
            authority_account_info,
            settings_account_info,
            system_program_info,
            Settings::LEN,
            settings_account_signer_seeds,
            rent,
        )?;

        let settings_account_data = Settings {
            is_initialized: true,
            name: name.clone(),
            kind,
            withdrawal_limit,
            deposit_limit,
            decimals,
            admin,
            token: token_account_info.key.clone()
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        if kind == TokenKind::Solana {
            // Create vault
            let vault_nonce = validate_vault_account(program_id, &name,token_account_info.key)?;
            let vault_account_signer_seeds: &[&[_]] = &[b"vault", &name, &[vault_nonce]];

            // TODO! initialize account here

        }

        Ok(())
    }
}

fn validate_authority(
    authority_account_info: &AccountInfo,
    program_account_info: &AccountInfo,
    program_buffer_account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if let UpgradeableLoaderState::Program {
        programdata_address,
    } =
        bincode::deserialize::<UpgradeableLoaderState>(&program_account_info.data.borrow()).unwrap()
    {
        if programdata_address == *program_buffer_account_info.key {
            if let UpgradeableLoaderState::ProgramData {
                upgrade_authority_address,
                ..
            } = bincode::deserialize::<UpgradeableLoaderState>(
                &program_buffer_account_info.data.borrow(),
            )
            .unwrap()
            {
                if upgrade_authority_address.unwrap() == *authority_account_info.key {
                    return Ok(());
                }
            }
        }
    }

    Err(ProgramError::MissingRequiredSignature)
}

fn validate_settings_account(
    program_id: &Pubkey,
    token_name: &str,
    settings_account: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(&[b"settings", token_name], program_id);

    if pda != *settings_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

fn validate_vault_account(
    program_id: &Pubkey,
    token_name: &str,
    vault_account: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(&[b"vault", token_name], program_id);

    if pda != *vault_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

fn create_account<'a>(
    program_id: &Pubkey,
    funder_account_info: &AccountInfo<'a>,
    new_account_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    data_len: usize,
    seeds: &[&[u8]],
    rent: &Rent,
) -> Result<(), ProgramError> {
    let required_lamports = rent
        .minimum_balance(data_len)
        .max(1)
        .saturating_sub(new_account_info.lamports());

    if required_lamports > 0 {
        msg!("Transfer {} lamports to the account", required_lamports);
        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                new_account_info.key,
                required_lamports,
            ),
            &[
                funder_account_info.clone(),
                new_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    msg!("Allocate space for the account");
    invoke_signed(
        &system_instruction::allocate(new_account_info.key, data_len as u64),
        &[new_account_info.clone(), system_program_info.clone()],
        &[seeds],
    )?;

    msg!("Assign the account to the round-loader program");
    invoke_signed(
        &system_instruction::assign(new_account_info.key, program_id),
        &[new_account_info.clone(), system_program_info.clone()],
        &[seeds],
    )?;

    Ok(())
}
