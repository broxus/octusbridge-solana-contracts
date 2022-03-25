use borsh::BorshDeserialize;

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::hash::Hash;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::{
    Deposit, EverAddress, Settings, TokenKind, TokenProxyError, TokenProxyInstruction, Withdrawal,
    WithdrawalEvent, WithdrawalMeta, WithdrawalPattern, WithdrawalStatus, WITHDRAWAL_PERIOD,
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
            TokenProxyInstruction::InitializeMint {
                name,
                decimals,
                deposit_limit,
                withdrawal_limit,
                withdrawal_daily_limit,
                admin,
            } => {
                msg!("Instruction: Initialize Mint");
                Self::process_mint_initialize(
                    program_id,
                    accounts,
                    name,
                    decimals,
                    deposit_limit,
                    withdrawal_limit,
                    withdrawal_daily_limit,
                    admin,
                )?;
            }
            TokenProxyInstruction::InitializeVault {
                name,
                decimals,
                deposit_limit,
                withdrawal_limit,
                withdrawal_daily_limit,
                admin,
            } => {
                msg!("Instruction: Initialize Vault");
                Self::process_vault_initialize(
                    program_id,
                    accounts,
                    name,
                    decimals,
                    deposit_limit,
                    withdrawal_limit,
                    withdrawal_daily_limit,
                    admin,
                )?;
            }
            TokenProxyInstruction::DepositEver {
                name,
                payload_id,
                recipient,
                amount,
            } => {
                msg!("Instruction: Deposit EVER");
                Self::process_deposit_ever(
                    program_id, accounts, name, payload_id, recipient, amount,
                )?;
            }
            TokenProxyInstruction::DepositSol {
                name,
                payload_id,
                recipient,
                amount,
            } => {
                msg!("Instruction: Deposit SOL");
                Self::process_deposit_sol(
                    program_id, accounts, name, payload_id, recipient, amount,
                )?;
            }
            TokenProxyInstruction::WithdrawRequest {
                name,
                payload_id,
                round_number,
                sender,
                timestamp,
                amount,
            } => {
                msg!("Instruction: Withdraw EVER/SOL request");
                Self::process_withdraw_request(
                    program_id,
                    accounts,
                    name,
                    payload_id,
                    round_number,
                    sender,
                    timestamp,
                    amount,
                )?;
            }
            TokenProxyInstruction::ConfirmWithdrawRequest {
                payload_id,
                round_number,
            } => {
                msg!("Instruction: Confirm Withdraw EVER/SOL request");
                Self::process_confirm_withdraw_request(
                    program_id,
                    accounts,
                    payload_id,
                    round_number,
                )?;
            }
            TokenProxyInstruction::UpdateWithdrawStatus { name, payload_id } => {
                msg!("Instruction: Update Withdraw status");
                Self::process_update_withdraw_status(program_id, accounts, name, payload_id)?;
            }
            TokenProxyInstruction::WithdrawEver { name, payload_id } => {
                msg!("Instruction: Withdraw EVER");
                Self::process_withdraw_ever(program_id, accounts, name, payload_id)?;
            }
            TokenProxyInstruction::WithdrawSol { name, payload_id } => {
                msg!("Instruction: Withdraw SOL");
                Self::process_withdraw_sol(program_id, accounts, name, payload_id)?;
            }
            TokenProxyInstruction::ApproveWithdrawEver { name, payload_id } => {
                msg!("Instruction: Approve Withdraw EVER");
                Self::process_approve_withdraw_ever(program_id, accounts, name, payload_id)?;
            }
            TokenProxyInstruction::ApproveWithdrawSol { name, payload_id } => {
                msg!("Instruction: Approve Withdraw SOL");
                Self::process_approve_withdraw_sol(program_id, accounts, name, payload_id)?;
            }
            TokenProxyInstruction::CancelWithdrawSol {
                payload_id,
                deposit_payload_id,
            } => {
                msg!("Instruction: Cancel Withdraw SOL");
                Self::process_cancel_withdraw_sol(
                    program_id,
                    accounts,
                    payload_id,
                    deposit_payload_id,
                )?;
            }
            TokenProxyInstruction::ForceWithdrawSol { name, payload_id } => {
                msg!("Instruction: Force Withdraw SOL");
                Self::process_force_withdraw_sol(program_id, accounts, name, payload_id)?;
            }
            TokenProxyInstruction::FillWithdrawSol {
                payload_id,
                deposit_payload_id,
                recipient,
            } => {
                msg!("Instruction: Fill Withdraw SOL");
                Self::process_fill_withdraw_sol(
                    program_id,
                    accounts,
                    payload_id,
                    deposit_payload_id,
                    recipient,
                )?;
            }
            TokenProxyInstruction::TransferFromVault { name, amount } => {
                msg!("Instruction: Transfer from Vault");
                Self::process_transfer_from_vault(program_id, accounts, name, amount)?;
            }
            TokenProxyInstruction::ChangeBountyForWithdrawSol { payload_id, bounty } => {
                msg!("Instruction: Change Bounty for Withdraw SOL");
                Self::process_change_bounty_for_withdraw_sol(
                    program_id, accounts, payload_id, bounty,
                )?;
            }
            TokenProxyInstruction::ChangeSettings {
                name,
                emergency,
                deposit_limit,
                withdrawal_limit,
                withdrawal_daily_limit,
            } => {
                msg!("Instruction: Change Settings");
                Self::process_change_settings(
                    program_id,
                    accounts,
                    name,
                    emergency,
                    deposit_limit,
                    withdrawal_limit,
                    withdrawal_daily_limit,
                )?;
            }
        };

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_mint_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        decimals: u8,
        deposit_limit: u64,
        withdrawal_limit: u64,
        withdrawal_daily_limit: u64,
        admin: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let initializer_account_info = next_account_info(account_info_iter)?;
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

        // Validate Initializer Account
        bridge_utils::validate_programdata_account(program_id, programdata_account_info.key)?;
        bridge_utils::validate_initializer_account(
            initializer_account_info.key,
            programdata_account_info,
        )?;

        // Validate Mint Account
        let mint_nonce = bridge_utils::validate_mint_account(program_id, &name, mint_account_info)?;
        let mint_account_signer_seeds: &[&[_]] = &[br"mint", name.as_bytes(), &[mint_nonce]];

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
                mint_account_info.key,
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

        // Validate Settings Account
        let settings_nonce =
            bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;
        let settings_account_signer_seeds: &[&[_]] =
            &[br"settings", name.as_bytes(), &[settings_nonce]];

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
            emergency: false,
            kind: TokenKind::Ever {
                mint: *mint_account_info.key,
            },
            withdrawal_daily_amount: 0,
            withdrawal_ttl: 0,
            decimals,
            deposit_limit,
            withdrawal_limit,
            withdrawal_daily_limit,
            admin,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_vault_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        decimals: u8,
        deposit_limit: u64,
        withdrawal_limit: u64,
        withdrawal_daily_limit: u64,
        admin: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let initializer_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
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

        // Validate Vault Account
        let vault_nonce =
            bridge_utils::validate_vault_account(program_id, &name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

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
                vault_account_info.key,
            )?,
            &[
                vault_account_info.clone(),
                token_program_info.clone(),
                rent_sysvar_info.clone(),
                mint_account_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        // Validate Settings Account
        let settings_nonce =
            bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;
        let settings_account_signer_seeds: &[&[_]] =
            &[br"settings", name.as_bytes(), &[settings_nonce]];

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
            emergency: false,
            kind: TokenKind::Solana {
                mint: *mint_account_info.key,
                vault: *vault_account_info.key,
            },
            withdrawal_daily_amount: 0,
            withdrawal_ttl: 0,
            decimals,
            deposit_limit,
            withdrawal_limit,
            withdrawal_daily_limit,
            admin,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_deposit_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
        recipient: EverAddress,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let authority_sender_account_info = next_account_info(account_info_iter)?;
        let token_sender_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !authority_sender_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        // Validate Mint Account
        bridge_utils::validate_mint_account(program_id, &name, mint_account_info)?;

        // Burn EVER tokens
        invoke(
            &spl_token::instruction::burn(
                token_program_info.key,
                token_sender_account_info.key,
                mint_account_info.key,
                authority_sender_account_info.key,
                &[authority_sender_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                authority_sender_account_info.clone(),
                token_sender_account_info.clone(),
                mint_account_info.clone(),
            ],
        )?;

        // Validate Deposit Account
        let deposit_nonce =
            bridge_utils::validate_deposit_account(program_id, &payload_id, deposit_account_info)?;
        let deposit_account_signer_seeds: &[&[_]] =
            &[br"deposit", &payload_id.to_bytes(), &[deposit_nonce]];

        // Create Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(Deposit::LEN)),
                Deposit::LEN as u64,
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
            sender: *authority_sender_account_info.key,
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

    fn process_deposit_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
        recipient: EverAddress,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let authority_sender_account_info = next_account_info(account_info_iter)?;
        let token_sender_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !authority_sender_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let (mint_account, vault_account) = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Mint Account
        if mint_account != mint_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Vault Account
        if vault_account != vault_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data.amount + amount > settings_account_data.deposit_limit {
            return Err(TokenProxyError::DepositLimit.into());
        }

        // Transfer SOL tokens to Vault Account
        invoke(
            &spl_token::instruction::transfer(
                token_program_info.key,
                token_sender_account_info.key,
                vault_account_info.key,
                authority_sender_account_info.key,
                &[authority_sender_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                authority_sender_account_info.clone(),
                token_sender_account_info.clone(),
                vault_account_info.clone(),
            ],
        )?;

        // Validate Deposit Account
        let deposit_nonce =
            bridge_utils::validate_deposit_account(program_id, &payload_id, deposit_account_info)?;
        let deposit_account_signer_seeds: &[&[_]] =
            &[br"deposit", &payload_id.to_bytes(), &[deposit_nonce]];

        // Create Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(Deposit::LEN)),
                Deposit::LEN as u64,
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
            sender: *authority_sender_account_info.key,
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

    fn process_withdraw_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
        round_number: u32,
        sender: EverAddress,
        timestamp: i64,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let kind = settings_account_data.kind;
        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        // Validate Relay Round Account
        let relay_round_account = round_loader::get_associated_relay_round_address(round_number);
        if relay_round_account != *relay_round_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let relay_round_account_data =
            round_loader::RelayRound::unpack(&relay_round_account_info.data.borrow())?;
        let required_votes = (relay_round_account_data.relays.len() * 2 / 3 + 1) as u32;

        if relay_round_account_data.round_number != round_number {
            return Err(TokenProxyError::InvalidRelayRound.into());
        }

        if relay_round_account_data.round_ttl <= clock.unix_timestamp {
            return Err(TokenProxyError::RelayRoundExpired.into());
        }

        // Validate Withdrawal Account
        let withdrawal_nonce = bridge_utils::validate_withdraw_account(
            program_id,
            &payload_id,
            withdrawal_account_info,
        )?;
        let withdrawal_account_signer_seeds: &[&[_]] =
            &[br"withdrawal", &payload_id.to_bytes(), &[withdrawal_nonce]];

        // Create Withdraw Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                withdrawal_account_info.key,
                1.max(rent.minimum_balance(Withdrawal::LEN)),
                Withdrawal::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
            &[withdrawal_account_signer_seeds],
        )?;

        let recipient_account_data =
            spl_token::state::Account::unpack(&recipient_account_info.data.borrow())?;

        // Init Withdraw Account
        let withdrawal_account_data = Withdrawal {
            is_initialized: true,
            payload_id,
            round_number,
            signers: vec![],
            required_votes,
            event: WithdrawalEvent::new(
                settings_account_data.decimals,
                recipient_account_data.owner,
                sender,
                timestamp,
                amount,
            ),
            meta: WithdrawalMeta::new(*authority_account_info.key, kind, WithdrawalStatus::New, 0),
        };

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_confirm_withdraw_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload_id: Hash,
        round_number: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let relay_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Relay Round Account
        let relay_round_account = round_loader::get_associated_relay_round_address(round_number);
        if relay_round_account != *relay_round_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let relay_round_account_data =
            round_loader::RelayRound::unpack(&relay_round_account_info.data.borrow())?;

        if relay_round_account_data.round_number != round_number {
            return Err(TokenProxyError::InvalidRelayRound.into());
        }

        if relay_round_account_data.round_ttl <= clock.unix_timestamp {
            return Err(TokenProxyError::RelayRoundExpired.into());
        }

        if !relay_round_account_data
            .relays
            .contains(relay_account_info.key)
        {
            return Err(TokenProxyError::InvalidRelay.into());
        }

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            WithdrawalPattern::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.round_number != round_number {
            return Err(TokenProxyError::InvalidRelayRound.into());
        }

        // Add signer
        withdrawal_account_data
            .signers
            .push(*relay_account_info.key);

        WithdrawalPattern::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_update_withdraw_status(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let settings_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.signers.len() as u32 >= withdrawal_account_data.required_votes {
            if withdrawal_account_data.meta.status == WithdrawalStatus::New {
                let current_timestamp = clock.unix_timestamp;

                // If current timestamp has expired
                if settings_account_data.withdrawal_ttl < current_timestamp {
                    settings_account_data.withdrawal_ttl = current_timestamp + WITHDRAWAL_PERIOD;
                    settings_account_data.withdrawal_daily_amount = 0;
                }

                if settings_account_data.withdrawal_limit >= withdrawal_account_data.event.amount
                    && settings_account_data.withdrawal_daily_limit
                        >= settings_account_data.withdrawal_daily_amount
                            + withdrawal_account_data.event.amount
                {
                    settings_account_data.withdrawal_daily_amount +=
                        withdrawal_account_data.event.amount;

                    withdrawal_account_data.meta.status = WithdrawalStatus::WaitingForRelease;
                } else {
                    withdrawal_account_data.meta.status = WithdrawalStatus::WaitingForApprove;
                }
            }
        }

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_withdraw_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let mint_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let settings_kind = settings_account_data
            .kind
            .as_ever()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        let withdrawal_kind = withdrawal_account_data
            .meta
            .kind
            .as_ever()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate connection between Settings and Withdrawal Accounts
        if settings_kind != withdrawal_kind {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Recipient Account
        if withdrawal_account_data.event.recipient != *recipient_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Mint Account
        let mint_nonce = bridge_utils::validate_mint_account(program_id, &name, mint_account_info)?;
        let mint_account_signer_seeds: &[&[_]] = &[br"mint", name.as_bytes(), &[mint_nonce]];

        // Validate status
        if withdrawal_account_data.meta.status != WithdrawalStatus::WaitingForRelease {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Mint EVER tokens to Recipient Account
        invoke_signed(
            &spl_token::instruction::mint_to(
                token_program_info.key,
                mint_account_info.key,
                recipient_account_info.key,
                mint_account_info.key,
                &[mint_account_info.key],
                withdrawal_account_data.event.amount,
            )?,
            &[
                token_program_info.clone(),
                mint_account_info.clone(),
                recipient_account_info.clone(),
                mint_account_info.clone(),
            ],
            &[mint_account_signer_seeds],
        )?;

        withdrawal_account_data.meta.status = WithdrawalStatus::Processed;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let vault_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let settings_kind = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        let withdrawal_kind = withdrawal_account_data
            .meta
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate connection between Settings and Withdrawal Accounts
        if settings_kind != withdrawal_kind {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Recipient Account
        if withdrawal_account_data.event.recipient != *recipient_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate status
        if withdrawal_account_data.meta.status != WithdrawalStatus::WaitingForRelease {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Vault Account
        let vault_nonce =
            bridge_utils::validate_vault_account(program_id, &name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data.amount >= withdrawal_account_data.event.amount {
            // Transfer tokens from Vault Account to Recipient Account
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    vault_account_info.key,
                    recipient_account_info.key,
                    vault_account_info.key,
                    &[vault_account_info.key],
                    withdrawal_account_data.event.amount,
                )?,
                &[
                    token_program_info.clone(),
                    vault_account_info.clone(),
                    recipient_account_info.clone(),
                ],
                &[vault_account_signer_seeds],
            )?;

            withdrawal_account_data.meta.status = WithdrawalStatus::Processed;
        } else {
            withdrawal_account_data.meta.status = WithdrawalStatus::Pending;
        }

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_approve_withdraw_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        if settings_account_data.admin != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        let settings_kind = settings_account_data
            .kind
            .as_ever()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.meta.status != WithdrawalStatus::WaitingForApprove {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        let withdrawal_kind = withdrawal_account_data
            .meta
            .kind
            .as_ever()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate connection between Settings and Withdrawal Accounts
        if settings_kind != withdrawal_kind {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Recipient Account
        if withdrawal_account_data.event.recipient != *recipient_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Mint Account
        let mint_nonce = bridge_utils::validate_mint_account(program_id, &name, mint_account_info)?;
        let mint_account_signer_seeds: &[&[_]] = &[br"mint", name.as_bytes(), &[mint_nonce]];

        // Mint EVER token to Recipient Account
        invoke_signed(
            &spl_token::instruction::mint_to(
                token_program_info.key,
                mint_account_info.key,
                recipient_account_info.key,
                mint_account_info.key,
                &[mint_account_info.key],
                withdrawal_account_data.event.amount,
            )?,
            &[
                token_program_info.clone(),
                mint_account_info.clone(),
                recipient_account_info.clone(),
                mint_account_info.clone(),
            ],
            &[mint_account_signer_seeds],
        )?;

        withdrawal_account_data.meta.status = WithdrawalStatus::Processed;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_approve_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        if settings_account_data.admin != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        let settings_kind = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.meta.status != WithdrawalStatus::WaitingForApprove {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        let withdrawal_kind = withdrawal_account_data
            .meta
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate connection between Settings and Withdrawal Accounts
        if settings_kind != withdrawal_kind {
            return Err(ProgramError::InvalidAccountData);
        }

        withdrawal_account_data.meta.status = WithdrawalStatus::Pending;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_cancel_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload_id: Hash,
        deposit_payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let new_deposit_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.meta.author != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        if withdrawal_account_data.meta.status != WithdrawalStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        withdrawal_account_data.meta.status = WithdrawalStatus::Cancelled;

        // Validate a new Deposit Account
        let deposit_nonce = bridge_utils::validate_deposit_account(
            program_id,
            &deposit_payload_id,
            new_deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_payload_id.to_bytes(),
            &[deposit_nonce],
        ];

        // Create a new Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                new_deposit_account_info.key,
                1.max(rent.minimum_balance(Deposit::LEN)),
                Deposit::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                new_deposit_account_info.clone(),
                system_program_info.clone(),
            ],
            &[deposit_account_signer_seeds],
        )?;

        // Init Deposit Account
        let deposit_account_data = Deposit {
            is_initialized: true,
            payload_id: deposit_payload_id,
            kind: withdrawal_account_data.meta.kind,
            sender: withdrawal_account_data.event.recipient,
            recipient: withdrawal_account_data.event.sender,
            decimals: withdrawal_account_data.event.decimals,
            amount: withdrawal_account_data.event.amount,
        };

        Deposit::pack(
            deposit_account_data,
            &mut new_deposit_account_info.data.borrow_mut(),
        )?;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_force_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        payload_id: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let vault_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let settings_kind = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.meta.status != WithdrawalStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        let withdrawal_kind = withdrawal_account_data
            .meta
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate connection between Settings and Withdrawal Accounts
        if settings_kind != withdrawal_kind {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Recipient Account
        if withdrawal_account_data.event.recipient != *recipient_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate Vault Account
        let vault_nonce =
            bridge_utils::validate_vault_account(program_id, &name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if withdrawal_account_data.event.amount > vault_account_data.amount {
            return Err(TokenProxyError::InsufficientVaultBalance.into());
        }

        // Transfer SOL tokens from Vault Account to Recipient Account
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                vault_account_info.key,
                recipient_account_info.key,
                vault_account_info.key,
                &[vault_account_info.key],
                withdrawal_account_data.event.amount,
            )?,
            &[
                token_program_info.clone(),
                vault_account_info.clone(),
                recipient_account_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        withdrawal_account_data.meta.status = WithdrawalStatus::Processed;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_fill_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload_id: Hash,
        deposit_payload_id: Hash,
        recipient: EverAddress,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let authority_sender_account_info = next_account_info(account_info_iter)?;
        let token_sender_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let new_deposit_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !authority_sender_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.meta.status != WithdrawalStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Recipient account
        if withdrawal_account_data.event.recipient != *recipient_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check balance
        let token_sender_account_data =
            spl_token::state::Account::unpack(&token_sender_account_info.data.borrow())?;

        if token_sender_account_data.amount
            < withdrawal_account_data.event.amount - withdrawal_account_data.meta.bounty
        {
            return Err(ProgramError::InsufficientFunds);
        }

        // Transfer SOL tokens
        invoke(
            &spl_token::instruction::transfer(
                token_program_info.key,
                token_sender_account_info.key,
                recipient_account_info.key,
                authority_sender_account_info.key,
                &[authority_sender_account_info.key],
                withdrawal_account_data.event.amount - withdrawal_account_data.meta.bounty,
            )?,
            &[
                token_program_info.clone(),
                authority_sender_account_info.clone(),
                token_sender_account_info.clone(),
                recipient_account_info.clone(),
            ],
        )?;

        withdrawal_account_data.meta.status = WithdrawalStatus::Processed;

        // Validate a new Deposit Account
        let deposit_nonce = bridge_utils::validate_deposit_account(
            program_id,
            &deposit_payload_id,
            new_deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_payload_id.to_bytes(),
            &[deposit_nonce],
        ];

        // Create a new Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                new_deposit_account_info.key,
                1.max(rent.minimum_balance(Deposit::LEN)),
                Deposit::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                new_deposit_account_info.clone(),
                system_program_info.clone(),
            ],
            &[deposit_account_signer_seeds],
        )?;

        // Init Deposit Account
        let deposit_account_data = Deposit {
            is_initialized: true,
            payload_id: deposit_payload_id,
            kind: withdrawal_account_data.meta.kind,
            sender: *authority_sender_account_info.key,
            recipient,
            decimals: withdrawal_account_data.event.decimals,
            amount: withdrawal_account_data.event.amount,
        };

        Deposit::pack(
            deposit_account_data,
            &mut new_deposit_account_info.data.borrow_mut(),
        )?;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_transfer_from_vault(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let recipient_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        if settings_account_data.admin != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        let (_, _) = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Vault Account
        let vault_nonce =
            bridge_utils::validate_vault_account(program_id, &name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data.amount < amount {
            return Err(TokenProxyError::InsufficientVaultBalance.into());
        }

        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                vault_account_info.key,
                recipient_account_info.key,
                vault_account_info.key,
                &[vault_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                vault_account_info.clone(),
                recipient_account_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        Ok(())
    }

    fn process_change_bounty_for_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload_id: Hash,
        bounty: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_withdraw_account(program_id, &payload_id, withdrawal_account_info)?;

        let mut withdrawal_account_data =
            Withdrawal::unpack(&withdrawal_account_info.data.borrow())?;

        if withdrawal_account_data.meta.status != WithdrawalStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        if withdrawal_account_data.meta.author != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        withdrawal_account_data.meta.bounty = bounty;

        Withdrawal::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_settings(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        emergency: bool,
        deposit_limit: u64,
        withdrawal_limit: u64,
        withdrawal_daily_limit: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_settings_account(program_id, &name, settings_account_info)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.admin != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        settings_account_data.emergency = emergency;
        settings_account_data.deposit_limit = deposit_limit;
        settings_account_data.withdrawal_limit = withdrawal_limit;
        settings_account_data.withdrawal_daily_limit = withdrawal_daily_limit;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }
}
