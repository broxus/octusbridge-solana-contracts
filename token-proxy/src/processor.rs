use borsh::BorshDeserialize;

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::hash::Hash;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::{Deposit, Settings, TokenKind, TokenProxyError, TokenProxyInstruction};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = TokenProxyInstruction::try_from_slice(instruction_data).unwrap();

        match instruction {
            TokenProxyInstruction::InitializeMint { name, decimals } => {
                msg!("Instruction: Initialize Mint");
                Self::process_mint_initialize(program_id, accounts, name, decimals)?;
            }
            TokenProxyInstruction::InitializeVault {
                deposit_limit,
                withdrawal_limit,
                decimals,
            } => {
                msg!("Instruction: Initialize Vault");
                Self::process_vault_initialize(
                    program_id,
                    accounts,
                    deposit_limit,
                    withdrawal_limit,
                    decimals,
                )?;
            }
            TokenProxyInstruction::DepositSolana {
                payload_id,
                recipient,
                amount,
            } => {
                msg!("Instruction: Deposit SOL");
                Self::process_deposit_solana(program_id, accounts, payload_id, recipient, amount)?;
            }
        };

        Ok(())
    }

    fn process_mint_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        decimals: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let initializer_account_info = next_account_info(account_info_iter)?;
        let mint_owner_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let spl_token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !initializer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate creator
        bridge_utils::validate_programdata_account(program_id, programdata_account_info.key)?;
        bridge_utils::validate_initializer_account(
            initializer_account_info.key,
            programdata_account_info,
        )?;

        // Validate Mint Account owner
        let (mint_owner_account, _nonce) = Pubkey::find_program_address(
            &[br"mint", &mint_account_info.key.to_bytes()],
            program_id,
        );
        if mint_owner_account != *mint_owner_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Mint Account
        let (mint_account, nonce) = Pubkey::find_program_address(&[name.as_bytes()], program_id);
        if mint_account != *mint_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let mint_account_signer_seeds: &[&[_]] = &[name.as_bytes(), &[nonce]];

        // Validate Settings Account
        let (settings_account, settings_nonce) = Pubkey::find_program_address(
            &[br"settings", &mint_account_info.key.to_bytes()],
            program_id,
        );
        if settings_account != *settings_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let settings_account_signer_seeds: &[&[_]] = &[
            br"settings",
            &mint_account_info.key.to_bytes(),
            &[settings_nonce],
        ];

        // Create Mint Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                mint_account_info.key,
                1.max(rent.minimum_balance(spl_token::state::Mint::LEN)),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            &[
                funder_account_info.clone(),
                mint_account_info.clone(),
                system_program_info.clone(),
            ],
            &[mint_account_signer_seeds],
        )?;

        // Init Mint Account
        invoke_signed(
            &spl_token::instruction::initialize_mint(
                &spl_token::id(),
                mint_account_info.key,
                mint_owner_account_info.key,
                None,
                decimals,
            )?,
            &[
                mint_account_info.clone(),
                spl_token_program_info.clone(),
                rent_sysvar_info.clone(),
            ],
            &[mint_account_signer_seeds],
        )?;

        // Create Settings Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                settings_account_info.key,
                1.max(rent.minimum_balance(Settings::LEN)),
                Settings::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                settings_account_info.clone(),
                system_program_info.clone(),
            ],
            &[settings_account_signer_seeds],
        )?;

        // Init Settings Account
        let settings_account_data = Settings {
            is_initialized: true,
            kind: TokenKind::Ever { name },
            decimals,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_vault_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_limit: u64,
        withdrawal_limit: u64,
        decimals: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let initializer_account_info = next_account_info(account_info_iter)?;
        let vault_owner_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !initializer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Initializer Account
        bridge_utils::validate_programdata_account(program_id, programdata_account_info.key)?;
        bridge_utils::validate_initializer_account(
            initializer_account_info.key,
            programdata_account_info,
        )?;

        // Validate Vault Account owner
        let (vault_owner_account, _nonce) = Pubkey::find_program_address(
            &[br"vault", &mint_account_info.key.to_bytes()],
            program_id,
        );
        if vault_owner_account != *vault_owner_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Vault Account
        let (vault_account, nonce) = Pubkey::find_program_address(
            &[
                &vault_owner_account.to_bytes(),
                &mint_account_info.key.to_bytes(),
            ],
            program_id,
        );
        if vault_account != *vault_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let vault_account_signer_seeds: &[&[_]] = &[
            &vault_owner_account.to_bytes(),
            &mint_account_info.key.to_bytes(),
            &[nonce],
        ];

        // Validate Settings Account
        let (settings_account, settings_nonce) = Pubkey::find_program_address(
            &[br"settings", &mint_account_info.key.to_bytes()],
            program_id,
        );
        if settings_account != *settings_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let settings_account_signer_seeds: &[&[_]] = &[
            br"settings",
            &mint_account_info.key.to_bytes(),
            &[settings_nonce],
        ];

        // Create Vault Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                vault_account_info.key,
                1.max(rent.minimum_balance(spl_token::state::Account::LEN)),
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            &[
                funder_account_info.clone(),
                vault_account_info.clone(),
                system_program_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        // Init Vault Account
        invoke_signed(
            &spl_token::instruction::initialize_account(
                &spl_token::id(),
                vault_account_info.key,
                mint_account_info.key,
                vault_owner_account_info.key,
            )?,
            &[
                vault_account_info.clone(),
                token_program_info.clone(),
                rent_sysvar_info.clone(),
                vault_owner_account_info.clone(),
                mint_account_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        // Create Settings Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                settings_account_info.key,
                1.max(rent.minimum_balance(Settings::LEN)),
                Settings::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                settings_account_info.clone(),
                system_program_info.clone(),
            ],
            &[settings_account_signer_seeds],
        )?;

        // Init Settings Account
        let settings_account_data = Settings {
            is_initialized: true,
            kind: TokenKind::Solana {
                mint: *mint_account_info.key,
                deposit_limit,
                withdrawal_limit,
            },
            decimals,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_deposit_solana(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload_id: Hash,
        recipient: Pubkey,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let sender_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidAccountData);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let (mint, deposit_limit, ..) = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Mint Account
        if mint_account_info.key != mint {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Vault account
        let (vault_owner_account, _nonce) = Pubkey::find_program_address(
            &[br"vault", &mint_account_info.key.to_bytes()],
            program_id,
        );

        let (vault_account, _nonce) = Pubkey::find_program_address(
            &[
                &vault_owner_account.to_bytes(),
                &mint_account_info.key.to_bytes(),
            ],
            program_id,
        );
        if vault_account != *vault_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Unpack Vault Account
        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        // Validate limits
        if vault_account_data.amount + amount > *deposit_limit {
            return Err(TokenProxyError::DepositLimit.into());
        }

        // Validate Deposit Account
        let (deposit_account, deposit_nonce) =
            Pubkey::find_program_address(&[br"deposit", &payload_id.to_bytes()], program_id);

        if deposit_account != *deposit_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let deposit_account_signer_seeds: &[&[_]] =
            &[br"deposit", &payload_id.to_bytes(), &[deposit_nonce]];

        // Transfer tokens
        invoke(
            &spl_token::instruction::transfer(
                token_program_info.key,
                sender_account_info.key,
                vault_account_info.key,
                authority_account_info.key,
                &[authority_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                authority_account_info.clone(),
                sender_account_info.clone(),
                vault_account_info.clone(),
            ],
        )?;

        // Create Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(Deposit::LEN)),
                Settings::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                deposit_account_info.clone(),
                system_program_info.clone(),
            ],
            &[deposit_account_signer_seeds],
        )?;

        // Init Deposit Account
        let deposit_account_data = Deposit {
            is_initialized: true,
            payload_id,
            kind: settings_account_data.kind,
            sender: *sender_account_info.key,
            recipient,
            decimals: settings_account_data.decimals,
            amount,
        };

        Deposit::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }
}
