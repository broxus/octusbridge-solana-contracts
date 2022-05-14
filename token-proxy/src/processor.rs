use borsh::BorshDeserialize;
use bridge_utils::state::{AccountKind, Proposal, PDA};
use bridge_utils::types::{EverAddress, Vote, RELAY_REPARATION};
use round_loader::RelayRound;

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::*;

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
                    deposit_limit,
                    withdrawal_limit,
                    withdrawal_daily_limit,
                    admin,
                )?;
            }
            TokenProxyInstruction::DepositEver {
                deposit_seed,
                recipient_address,
                amount,
            } => {
                msg!("Instruction: Deposit EVER");
                Self::process_deposit_ever(
                    program_id,
                    accounts,
                    deposit_seed,
                    recipient_address,
                    amount,
                )?;
            }
            TokenProxyInstruction::DepositSol {
                deposit_seed,
                recipient_address,
                amount,
            } => {
                msg!("Instruction: Deposit SOL");
                Self::process_deposit_sol(
                    program_id,
                    accounts,
                    deposit_seed,
                    recipient_address,
                    amount,
                )?;
            }
            TokenProxyInstruction::WithdrawRequest {
                event_timestamp,
                event_transaction_lt,
                sender_address,
                event_configuration,
                recipient_address,
                amount,
            } => {
                msg!("Instruction: Withdraw EVER/SOL request");
                Self::process_withdraw_request(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    event_configuration,
                    sender_address,
                    recipient_address,
                    amount,
                )?;
            }
            TokenProxyInstruction::VoteForWithdrawRequest { vote } => {
                msg!("Instruction: Vote for Withdraw EVER/SOL request");
                Self::process_vote_for_withdraw_request(program_id, accounts, vote)?;
            }
            TokenProxyInstruction::UpdateWithdrawStatus => {
                msg!("Instruction: Update Withdraw status");
                Self::process_update_withdraw_status(program_id, accounts)?;
            }
            TokenProxyInstruction::WithdrawEver => {
                msg!("Instruction: Withdraw EVER");
                Self::process_withdraw_ever(program_id, accounts)?;
            }
            TokenProxyInstruction::WithdrawSol => {
                msg!("Instruction: Withdraw SOL");
                Self::process_withdraw_sol(program_id, accounts)?;
            }
            TokenProxyInstruction::ApproveWithdrawEver => {
                msg!("Instruction: Approve Withdraw EVER");
                Self::process_approve_withdraw_ever(program_id, accounts)?;
            }
            TokenProxyInstruction::ApproveWithdrawSol => {
                msg!("Instruction: Approve Withdraw SOL");
                Self::process_approve_withdraw_sol(program_id, accounts)?;
            }
            TokenProxyInstruction::CancelWithdrawSol { deposit_seed } => {
                msg!("Instruction: Cancel Withdraw SOL");
                Self::process_cancel_withdraw_sol(program_id, accounts, deposit_seed)?;
            }
            TokenProxyInstruction::ForceWithdrawSol => {
                msg!("Instruction: Force Withdraw SOL");
                Self::process_force_withdraw_sol(program_id, accounts)?;
            }
            TokenProxyInstruction::FillWithdrawSol {
                deposit_seed,
                recipient_address,
            } => {
                msg!("Instruction: Fill Withdraw SOL");
                Self::process_fill_withdraw_sol(
                    program_id,
                    accounts,
                    deposit_seed,
                    recipient_address,
                )?;
            }
            TokenProxyInstruction::TransferFromVault { amount } => {
                msg!("Instruction: Transfer from Vault");
                Self::process_transfer_from_vault(program_id, accounts, amount)?;
            }
            TokenProxyInstruction::ChangeBountyForWithdrawSol { bounty } => {
                msg!("Instruction: Change Bounty for Withdraw SOL");
                Self::process_change_bounty_for_withdraw_sol(program_id, accounts, bounty)?;
            }
            TokenProxyInstruction::ChangeSettings {
                emergency,
                deposit_limit,
                withdrawal_limit,
                withdrawal_daily_limit,
            } => {
                msg!("Instruction: Change Settings");
                Self::process_change_settings(
                    program_id,
                    accounts,
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
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            initializer_account_info.key,
            programdata_account_info,
        )?;

        // Check asset name length
        if name.len() > MAX_NAME_LEN {
            return Err(TokenProxyError::TokenNameLenLimit.into());
        }

        // Validate Mint Account
        let mint_nonce = validate_mint_account(program_id, &name, mint_account_info)?;
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
        let settings_nonce = validate_settings_account(program_id, &name, settings_account_info)?;
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
            account_kind: AccountKind::Settings,
            emergency: false,
            kind: TokenKind::Ever {
                mint: *mint_account_info.key,
            },
            withdrawal_daily_amount: 0,
            withdrawal_ttl: 0,
            name,
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
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            initializer_account_info.key,
            programdata_account_info,
        )?;

        // Check asset name length
        if name.len() > MAX_NAME_LEN {
            return Err(TokenProxyError::TokenNameLenLimit.into());
        }

        // Validate Vault Account
        let vault_nonce = validate_vault_account(program_id, &name, vault_account_info)?;
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
        let settings_nonce = validate_settings_account(program_id, &name, settings_account_info)?;
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
            account_kind: AccountKind::Settings,
            emergency: false,
            kind: TokenKind::Solana {
                mint: *mint_account_info.key,
                vault: *vault_account_info.key,
            },
            withdrawal_daily_amount: 0,
            withdrawal_ttl: 0,
            name,
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
        deposit_seed: u128,
        recipient_address: EverAddress,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let creator_token_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        // Validate Setting Account
        let name = &settings_account_data.name;
        validate_settings_account(program_id, name, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Mint Account
        validate_mint_account(program_id, name, mint_account_info)?;

        if mint_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        // Burn EVER tokens
        invoke(
            &spl_token::instruction::burn(
                &spl_token::id(),
                creator_token_account_info.key,
                mint_account_info.key,
                creator_account_info.key,
                &[creator_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                creator_account_info.clone(),
                creator_token_account_info.clone(),
                mint_account_info.clone(),
            ],
        )?;

        // Validate Deposit account
        let deposit_nonce = validate_deposit_account(
            program_id,
            deposit_seed,
            settings_account_info.key,
            deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &settings_account_info.key.to_bytes(),
            &[deposit_nonce],
        ];

        // Create Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(DepositToken::LEN)),
                DepositToken::LEN as u64,
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
        let deposit_account_data = DepositToken {
            is_initialized: true,
            account_kind: AccountKind::Deposit,
            event: DepositTokenEventWithLen::new(
                *creator_account_info.key,
                amount,
                recipient_address,
            ),
        };

        DepositToken::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_deposit_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        recipient_address: EverAddress,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let creator_token_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        // Validate Setting Account
        let name = &settings_account_data.name;
        validate_settings_account(program_id, name, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let (mint_account, vault_account) = settings_account_data
            .kind
            .as_solana()
            .ok_or(TokenProxyError::InvalidTokenKind)?;

        // Validate Mint Account
        if mint_account_info.key != mint_account && mint_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Vault Account
        if vault_account_info.key != vault_account && vault_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data
            .amount
            .checked_add(amount)
            .ok_or(TokenProxyError::ArithmeticsError)?
            > settings_account_data.deposit_limit
        {
            return Err(TokenProxyError::DepositLimit.into());
        }

        // Transfer SOL tokens to Vault Account
        invoke(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                creator_token_account_info.key,
                vault_account_info.key,
                creator_account_info.key,
                &[creator_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                creator_account_info.clone(),
                creator_token_account_info.clone(),
                vault_account_info.clone(),
            ],
        )?;

        // Validate Deposit account
        let deposit_nonce = validate_deposit_account(
            program_id,
            deposit_seed,
            settings_account_info.key,
            deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &settings_account_info.key.to_bytes(),
            &[deposit_nonce],
        ];

        // Create Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(DepositToken::LEN)),
                DepositToken::LEN as u64,
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
        let deposit_account_data = DepositToken {
            is_initialized: true,
            account_kind: AccountKind::Deposit,
            event: DepositTokenEventWithLen::new(
                *creator_account_info.key,
                amount,
                recipient_address,
            ),
        };

        DepositToken::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_withdraw_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
        event_configuration: Pubkey,
        sender_address: EverAddress,
        recipient_address: Pubkey,
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let author_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Setting Account
        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let name = &settings_account_data.name;
        validate_settings_account(program_id, name, settings_account_info)?;

        // Validate Relay Round Account
        if relay_round_account_info.owner != &round_loader::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;

        let round_number = relay_round_account_data.round_number;
        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
            relay_round_account_info,
        )?;

        if relay_round_account_data.round_end <= clock.unix_timestamp as u32 {
            return Err(TokenProxyError::RelayRoundExpired.into());
        }

        let required_votes = (relay_round_account_data.relays.len() * 2 / 3 + 1) as u32;

        // Validate Withdrawal Account
        let withdrawal_nonce = bridge_utils::helper::validate_proposal_account(
            program_id,
            author_account_info.key,
            settings_account_info.key,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;
        let withdrawal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &author_account_info.key.to_bytes(),
            &settings_account_info.key.to_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &[withdrawal_nonce],
        ];

        // Create Withdraw Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                withdrawal_account_info.key,
                1.max(rent.minimum_balance(WithdrawalToken::LEN)),
                WithdrawalToken::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
            &[withdrawal_account_signer_seeds],
        )?;

        // Init Withdraw Account
        let withdrawal_account_data = WithdrawalToken {
            is_initialized: true,
            round_number,
            required_votes,
            account_kind: AccountKind::Proposal,
            event: WithdrawalTokenEventWithLen::new(sender_address, amount, recipient_address),
            meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0),
            signers: vec![Vote::None; relay_round_account_data.relays.len()],
            pda: PDA {
                author: *author_account_info.key,
                settings: *settings_account_info.key,
                event_timestamp,
                event_transaction_lt,
                event_configuration,
            },
        };

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        // Send voting reparation for Relay to withdrawal account
        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                withdrawal_account_info.key,
                RELAY_REPARATION * relay_round_account_data.relays.len() as u64,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        Ok(())
    }

    fn process_vote_for_withdraw_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        vote: Vote,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let relay_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            Proposal::unpack_from_slice(&withdrawal_account_info.data.borrow())?;

        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        let round_number = withdrawal_account_data.round_number;

        // Validate Relay Round Account
        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
            relay_round_account_info,
        )?;

        if relay_round_account_info.owner != &round_loader::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;

        // Vote for withdraw request
        let index = relay_round_account_data
            .relays
            .iter()
            .position(|pubkey| pubkey == relay_account_info.key)
            .ok_or(TokenProxyError::InvalidRelay)?;
        withdrawal_account_data.signers[index] = vote;

        withdrawal_account_data.pack_into_slice(&mut withdrawal_account_info.data.borrow_mut());

        // Get back voting reparation to Relay
        **withdrawal_account_info.try_borrow_mut_lamports()? -= RELAY_REPARATION;
        **relay_account_info.try_borrow_mut_lamports()? += RELAY_REPARATION;

        Ok(())
    }

    fn process_update_withdraw_status(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let settings_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        // Do we have enough signers.
        let sig_count = withdrawal_account_data
            .signers
            .iter()
            .filter(|vote| **vote == Vote::Confirm)
            .count() as u32;

        if sig_count >= withdrawal_account_data.required_votes
            && withdrawal_account_data.meta.data.status == WithdrawalTokenStatus::New
        {
            // Validate Settings Account
            if *settings_account_info.key != settings {
                return Err(ProgramError::InvalidArgument);
            }

            let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

            let current_timestamp = clock.unix_timestamp;

            // If current timestamp has expired
            if settings_account_data.withdrawal_ttl < current_timestamp {
                settings_account_data.withdrawal_ttl = current_timestamp + WITHDRAWAL_TOKEN_PERIOD;
                settings_account_data.withdrawal_daily_amount = 0;
            }

            if settings_account_data.withdrawal_limit >= withdrawal_account_data.event.data.amount
                && settings_account_data.withdrawal_daily_limit
                    >= settings_account_data
                        .withdrawal_daily_amount
                        .checked_add(withdrawal_account_data.event.data.amount)
                        .ok_or(TokenProxyError::ArithmeticsError)?
            {
                settings_account_data.withdrawal_daily_amount = settings_account_data
                    .withdrawal_daily_amount
                    .checked_add(withdrawal_account_data.event.data.amount)
                    .ok_or(TokenProxyError::ArithmeticsError)?;

                withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::WaitingForRelease;
            } else {
                withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::WaitingForApprove;
            }

            Settings::pack(
                settings_account_data,
                &mut settings_account_info.data.borrow_mut(),
            )?;
        }

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_withdraw_ever(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let mint_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let name = &settings_account_data.name;

        // Validate Recipient Account
        if recipient_token_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let recipient_token_account_data =
            spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

        let recipient_token_address = Pubkey::new_from_array(
            withdrawal_account_data
                .event
                .data
                .recipient_address
                .clone()
                .try_into()
                .map_err(|_| TokenProxyError::ConstructPubkey)?,
        );
        if recipient_token_account_data.owner != recipient_token_address {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Mint Account
        let mint_nonce = validate_mint_account(program_id, name, mint_account_info)?;
        let mint_account_signer_seeds: &[&[_]] = &[br"mint", name.as_bytes(), &[mint_nonce]];

        if mint_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate status
        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::WaitingForRelease {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Mint EVER tokens to Recipient Account
        invoke_signed(
            &spl_token::instruction::mint_to(
                &spl_token::id(),
                mint_account_info.key,
                recipient_token_account_info.key,
                mint_account_info.key,
                &[mint_account_info.key],
                withdrawal_account_data.event.data.amount,
            )?,
            &[
                token_program_info.clone(),
                mint_account_info.clone(),
                recipient_token_account_info.clone(),
                mint_account_info.clone(),
            ],
            &[mint_account_signer_seeds],
        )?;

        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_withdraw_sol(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let vault_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let name = &settings_account_data.name;

        // Validate Recipient Account
        let recipient_token_account_data =
            spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

        if recipient_token_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let recipient_token_address = Pubkey::new_from_array(
            withdrawal_account_data
                .event
                .data
                .recipient_address
                .clone()
                .try_into()
                .map_err(|_| TokenProxyError::ConstructPubkey)?,
        );
        if recipient_token_account_data.owner != recipient_token_address {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate status
        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::WaitingForRelease {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Vault Account
        let vault_nonce = validate_vault_account(program_id, name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

        if vault_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data.amount >= withdrawal_account_data.event.data.amount {
            // Transfer tokens from Vault Account to Recipient Account
            invoke_signed(
                &spl_token::instruction::transfer(
                    &spl_token::id(),
                    vault_account_info.key,
                    recipient_token_account_info.key,
                    vault_account_info.key,
                    &[vault_account_info.key],
                    withdrawal_account_data.event.data.amount,
                )?,
                &[
                    token_program_info.clone(),
                    vault_account_info.clone(),
                    recipient_token_account_info.clone(),
                ],
                &[vault_account_signer_seeds],
            )?;

            withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;
        } else {
            withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;
        }

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_approve_withdraw_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let admin_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !admin_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::WaitingForApprove {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        if settings_account_data.admin != *admin_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        let name = &settings_account_data.name;

        // Validate Recipient Account
        let recipient_token_account_data =
            spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

        if recipient_token_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let recipient_token_address = Pubkey::new_from_array(
            withdrawal_account_data
                .event
                .data
                .recipient_address
                .clone()
                .try_into()
                .map_err(|_| TokenProxyError::ConstructPubkey)?,
        );
        if recipient_token_account_data.owner != recipient_token_address {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Mint Account
        let mint_nonce = validate_mint_account(program_id, name, mint_account_info)?;
        let mint_account_signer_seeds: &[&[_]] = &[br"mint", name.as_bytes(), &[mint_nonce]];

        if mint_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        // Mint EVER token to Recipient Account
        invoke_signed(
            &spl_token::instruction::mint_to(
                &spl_token::id(),
                mint_account_info.key,
                recipient_token_account_info.key,
                mint_account_info.key,
                &[mint_account_info.key],
                withdrawal_account_data.event.data.amount,
            )?,
            &[
                token_program_info.clone(),
                mint_account_info.clone(),
                recipient_token_account_info.clone(),
                mint_account_info.clone(),
            ],
            &[mint_account_signer_seeds],
        )?;

        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_approve_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let admin_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;

        if !admin_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::WaitingForApprove {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        if settings_account_data.admin != *admin_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_cancel_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let author_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let new_deposit_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        if author != *author_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Cancelled;

        // Validate a new Deposit account
        let deposit_nonce = validate_deposit_account(
            program_id,
            deposit_seed,
            settings_account_info.key,
            new_deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &settings_account_info.key.to_bytes(),
            &[deposit_nonce],
        ];

        // Create a new Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                new_deposit_account_info.key,
                1.max(rent.minimum_balance(DepositToken::LEN)),
                DepositToken::LEN as u64,
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
        let sender_address = Pubkey::new_from_array(
            withdrawal_account_data
                .event
                .data
                .recipient_address
                .clone()
                .try_into()
                .map_err(|_| TokenProxyError::ConstructPubkey)?,
        );

        let deposit_account_data = DepositToken {
            is_initialized: true,
            account_kind: AccountKind::Deposit,
            event: DepositTokenEventWithLen::new(
                sender_address,
                withdrawal_account_data.event.data.amount,
                withdrawal_account_data.event.data.sender_address,
            ),
        };

        DepositToken::pack(
            deposit_account_data,
            &mut new_deposit_account_info.data.borrow_mut(),
        )?;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_force_withdraw_sol(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let vault_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        let name = &settings_account_data.name;

        // Validate Recipient Account
        let recipient_token_account_data =
            spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

        if recipient_token_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let recipient_token_address = Pubkey::new_from_array(
            withdrawal_account_data
                .event
                .data
                .recipient_address
                .clone()
                .try_into()
                .map_err(|_| TokenProxyError::ConstructPubkey)?,
        );

        if recipient_token_account_data.owner != recipient_token_address {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Vault Account
        let vault_nonce = validate_vault_account(program_id, name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

        if vault_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if withdrawal_account_data.event.data.amount > vault_account_data.amount {
            return Err(TokenProxyError::InsufficientVaultBalance.into());
        }

        // Transfer SOL tokens from Vault Account to Recipient Account
        invoke_signed(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                vault_account_info.key,
                recipient_token_account_info.key,
                vault_account_info.key,
                &[vault_account_info.key],
                withdrawal_account_data.event.data.amount,
            )?,
            &[
                token_program_info.clone(),
                vault_account_info.clone(),
                recipient_token_account_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_fill_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        recipient_address: EverAddress,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let author_account_info = next_account_info(account_info_iter)?;
        let author_token_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let new_deposit_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        // Validate Setting Account
        if *settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.emergency {
            return Err(TokenProxyError::EmergencyEnabled.into());
        }

        // Validate Recipient account
        let recipient_token_account_data =
            spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

        if recipient_token_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let recipient_token_address = Pubkey::new_from_array(
            withdrawal_account_data
                .event
                .data
                .recipient_address
                .clone()
                .try_into()
                .map_err(|_| TokenProxyError::ConstructPubkey)?,
        );

        if recipient_token_account_data.owner != recipient_token_address {
            return Err(ProgramError::InvalidAccountData);
        }

        // Transfer SOL tokens
        invoke(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                author_token_account_info.key,
                recipient_token_account_info.key,
                author_account_info.key,
                &[author_account_info.key],
                withdrawal_account_data
                    .event
                    .data
                    .amount
                    .checked_sub(withdrawal_account_data.meta.data.bounty)
                    .ok_or(TokenProxyError::ArithmeticsError)?,
            )?,
            &[
                token_program_info.clone(),
                author_account_info.clone(),
                author_token_account_info.clone(),
                recipient_token_account_info.clone(),
            ],
        )?;

        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;

        // Validate a new Deposit account
        let deposit_nonce = validate_deposit_account(
            program_id,
            deposit_seed,
            settings_account_info.key,
            new_deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &settings_account_info.key.to_bytes(),
            &[deposit_nonce],
        ];

        // Create a new Deposit Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                new_deposit_account_info.key,
                1.max(rent.minimum_balance(DepositToken::LEN)),
                DepositToken::LEN as u64,
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
        let deposit_account_data = DepositToken {
            is_initialized: true,
            account_kind: AccountKind::Deposit,
            event: DepositTokenEventWithLen::new(
                *author_account_info.key,
                withdrawal_account_data.event.data.amount,
                recipient_address,
            ),
        };

        DepositToken::pack(
            deposit_account_data,
            &mut new_deposit_account_info.data.borrow_mut(),
        )?;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_transfer_from_vault(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let admin_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if !admin_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.admin != *admin_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        // Validate Setting Account
        let name = &settings_account_data.name;
        validate_settings_account(program_id, name, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Vault Account
        let vault_nonce = validate_vault_account(program_id, name, vault_account_info)?;
        let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

        if vault_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data.amount < amount {
            return Err(TokenProxyError::InsufficientVaultBalance.into());
        }

        invoke_signed(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                vault_account_info.key,
                recipient_token_account_info.key,
                vault_account_info.key,
                &[vault_account_info.key],
                amount,
            )?,
            &[
                token_program_info.clone(),
                vault_account_info.clone(),
                recipient_token_account_info.clone(),
            ],
            &[vault_account_signer_seeds],
        )?;

        Ok(())
    }

    fn process_change_bounty_for_withdraw_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        bounty: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let author_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Withdrawal Account
        if withdrawal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut withdrawal_account_data =
            WithdrawalToken::unpack(&withdrawal_account_info.data.borrow())?;
        let author = withdrawal_account_data.pda.author;
        let settings = withdrawal_account_data.pda.settings;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::Pending {
            return Err(TokenProxyError::InvalidWithdrawalStatus.into());
        }

        if withdrawal_account_data.pda.author != *author_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        withdrawal_account_data.meta.data.bounty = bounty;

        WithdrawalToken::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_settings(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
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

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if settings_account_data.admin != *authority_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        // Validate Setting Account
        let name = &settings_account_data.name;
        validate_settings_account(program_id, name, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
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
