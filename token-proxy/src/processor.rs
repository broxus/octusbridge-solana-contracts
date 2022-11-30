use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::errors::SolanaBridgeError;
use bridge_utils::state::{AccountKind, Proposal, PDA};
use bridge_utils::types::{EverAddress, Vote, RELAY_REPARATION};
use round_loader::RelayRound;

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::{Clock, SECONDS_PER_DAY};
use solana_program::entrypoint::ProgramResult;
use solana_program::hash::hash;
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
        let instruction = TokenProxyInstruction::try_from_slice(instruction_data)?;

        match instruction {
            TokenProxyInstruction::Initialize {
                guardian,
                manager,
                withdrawal_manager,
            } => {
                msg!("Instruction: Initialize Token Proxy");
                Self::process_initialize(
                    program_id,
                    accounts,
                    guardian,
                    manager,
                    withdrawal_manager,
                )?;
            }
            TokenProxyInstruction::DepositMultiTokenEver {
                deposit_seed,
                recipient,
                amount,
                sol_amount,
                payload,
            } => {
                msg!("Instruction: Deposit MULTI TOKEN EVER");
                Self::process_deposit_multi_token_ever(
                    program_id,
                    accounts,
                    deposit_seed,
                    recipient,
                    amount,
                    sol_amount,
                    payload,
                )?;
            }
            TokenProxyInstruction::DepositMultiTokenSol {
                deposit_seed,
                recipient,
                amount,
                name,
                symbol,
                sol_amount,
                payload,
            } => {
                msg!("Instruction: Deposit MULTI TOKEN SOL");
                Self::process_deposit_multi_token_sol(
                    program_id,
                    accounts,
                    deposit_seed,
                    name,
                    symbol,
                    recipient,
                    amount,
                    sol_amount,
                    payload,
                )?;
            }
            TokenProxyInstruction::WithdrawMultiTokenEverRequest {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
                token,
                name,
                symbol,
                decimals,
                recipient,
                amount,
            } => {
                msg!("Instruction: Withdraw Multi token EVER request");
                Self::process_withdraw_multi_token_ever_request(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    event_configuration,
                    token,
                    name,
                    symbol,
                    decimals,
                    recipient,
                    amount,
                )?;
            }
            TokenProxyInstruction::WithdrawMultiTokenSolRequest {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
                recipient,
                amount,
            } => {
                msg!("Instruction: Withdraw multi token SOL request");
                Self::process_withdraw_multi_token_sol_request(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    event_configuration,
                    recipient,
                    amount,
                )?;
            }
            TokenProxyInstruction::VoteForWithdrawRequest { vote } => {
                msg!("Instruction: Vote for Withdraw EVER/SOL request");
                Self::process_vote_for_withdraw_request(program_id, accounts, vote)?;
            }
            TokenProxyInstruction::WithdrawMultiTokenEver => {
                msg!("Instruction: Withdraw Multi Token EVER");
                Self::process_withdraw_multi_token_ever(program_id, accounts)?;
            }
            TokenProxyInstruction::WithdrawMultiTokenSol => {
                msg!("Instruction: Withdraw Multi Token SOL");
                Self::process_withdraw_multi_token_sol(program_id, accounts)?;
            }
            TokenProxyInstruction::ChangeGuardian { new_guardian } => {
                msg!("Instruction: Update guardian");
                Self::process_change_guardian(program_id, accounts, new_guardian)?;
            }
            TokenProxyInstruction::ChangeManager { new_manager } => {
                msg!("Instruction: Update manager");
                Self::process_change_manager(program_id, accounts, new_manager)?;
            }
            TokenProxyInstruction::ChangeWithdrawalManager {
                new_withdrawal_manager,
            } => {
                msg!("Instruction: Update withdrawal manager");
                Self::process_change_withdrawal_manager(
                    program_id,
                    accounts,
                    new_withdrawal_manager,
                )?;
            }
            TokenProxyInstruction::ChangeDepositLimit { new_deposit_limit } => {
                msg!("Instruction: Update deposit limit");
                Self::process_change_deposit_limit(program_id, accounts, new_deposit_limit)?;
            }
            TokenProxyInstruction::ChangeWithdrawalLimits {
                new_withdrawal_limit,
                new_withdrawal_daily_limit,
            } => {
                msg!("Instruction: Update withdrawal limits");
                Self::process_change_withdrawal_limits(
                    program_id,
                    accounts,
                    new_withdrawal_limit,
                    new_withdrawal_daily_limit,
                )?;
            }
            TokenProxyInstruction::EnableEmergencyMode => {
                msg!("Instruction: Enable emergency mode");
                Self::process_enable_emergency_mode(program_id, accounts)?;
            }
            TokenProxyInstruction::DisableEmergencyMode => {
                msg!("Instruction: Disable emergency mode");
                Self::process_disable_emergency_mode(program_id, accounts)?;
            }
            TokenProxyInstruction::EnableTokenEmergencyMode => {
                msg!("Instruction: Enable token emergency mode");
                Self::process_enable_token_emergency_mode(program_id, accounts)?;
            }
            TokenProxyInstruction::DisableTokenEmergencyMode => {
                msg!("Instruction: Disable token emergency mode");
                Self::process_disable_token_emergency_mode(program_id, accounts)?;
            }
            TokenProxyInstruction::ApproveWithdrawEver => {
                msg!("Instruction: Approve Withdraw Multi Token EVER");
                Self::process_approve_withdraw_ever(program_id, accounts)?;
            }
            TokenProxyInstruction::ApproveWithdrawSol => {
                msg!("Instruction: Approve Withdraw Multi Token SOL");
                Self::process_approve_withdraw_sol(program_id, accounts)?;
            }
        };

        Ok(())
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        guardian: Pubkey,
        manager: Pubkey,
        withdrawal_manager: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let initializer_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let multi_vault_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
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

        // Create Settings Account
        let (settings_pubkey, settings_nonce) =
            Pubkey::find_program_address(&[br"settings"], program_id);
        let settings_account_signer_seeds: &[&[_]] = &[br"settings", &[settings_nonce]];

        if settings_pubkey != *settings_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

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
            account_kind: AccountKind::Settings(settings_nonce),
            emergency: false,
            guardian,
            manager,
            withdrawal_manager,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Create Multi Vault Account
        let (multi_vault_pubkey, multi_vault_nonce) =
            Pubkey::find_program_address(&[br"multivault"], program_id);
        let multi_vault_account_signer_seeds: &[&[_]] = &[br"multivault", &[multi_vault_nonce]];

        if multi_vault_pubkey != *multi_vault_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                multi_vault_account_info.key,
                1.max(rent.minimum_balance(MultiVault::LEN)),
                MultiVault::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                multi_vault_account_info.clone(),
                system_program_info.clone(),
            ],
            &[multi_vault_account_signer_seeds],
        )?;

        // Init Multi Vault Account
        let multi_vault_account_data = MultiVault {
            is_initialized: true,
            account_kind: AccountKind::MultiVault(multi_vault_nonce),
        };

        MultiVault::pack(
            multi_vault_account_data,
            &mut multi_vault_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_deposit_multi_token_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        recipient: EverAddress,
        amount: u64,
        sol_amount: u64,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let creator_token_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let multi_vault_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Token Settings Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (_, token, ever_decimals) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
        let (token_settings_nonce, mint_nonce) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Mint Account
        validate_mint_account(program_id, &token, mint_nonce, mint_account_info)?;

        let mint_account_data = spl_token::state::Mint::unpack(&mint_account_info.data.borrow())?;
        let solana_decimals = mint_account_data.decimals;

        // Validate Multi Vault Account
        let multi_vault_account_data = MultiVault::unpack(&multi_vault_account_info.data.borrow())?;
        let multi_vault_nonce = multi_vault_account_data
            .account_kind
            .into_multi_vault()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_multi_vault_account(program_id, multi_vault_nonce, multi_vault_account_info)?;

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

        // Create Deposit Account
        let (deposit_pubkey, deposit_nonce) =
            Pubkey::find_program_address(&[br"deposit", &deposit_seed.to_le_bytes()], program_id);
        let deposit_account_signer_seeds: &[&[_]] =
            &[br"deposit", &deposit_seed.to_le_bytes(), &[deposit_nonce]];

        if deposit_pubkey != *deposit_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(DepositMultiTokenEver::LEN)),
                DepositMultiTokenEver::LEN as u64,
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
        let fee_info = &mut token_settings_account_data.fee_info;

        let fee = 1.max(
            amount
                .checked_div(fee_info.divisor)
                .ok_or(SolanaBridgeError::Overflow)?
                .checked_mul(fee_info.multiplier)
                .ok_or(SolanaBridgeError::Overflow)?,
        );

        // Increase fee supply
        fee_info.supply = fee_info
            .supply
            .checked_add(fee)
            .ok_or(SolanaBridgeError::Overflow)?;

        // Amount without fee
        let pure_amount = amount.checked_sub(fee).ok_or(SolanaBridgeError::Overflow)?;

        // Amount in Ever decimals
        let transfer_amount = get_deposit_amount(pure_amount, ever_decimals, solana_decimals);

        let deposit_account_data = DepositMultiTokenEver {
            is_initialized: true,
            account_kind: AccountKind::Deposit(deposit_nonce),
            event: DepositMultiTokenEverEventWithLen::new(
                token,
                transfer_amount,
                sol_amount,
                recipient,
                payload,
            ),
            meta: DepositTokenMetaWithLen::new(deposit_seed),
        };

        DepositMultiTokenEver::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
        )?;

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        // Send SOL amount to multi vault
        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                multi_vault_account_info.key,
                sol_amount,
            ),
            &[
                funder_account_info.clone(),
                multi_vault_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_deposit_multi_token_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        name: String,
        symbol: String,
        recipient: EverAddress,
        amount: u64,
        sol_amount: u64,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let creator_token_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let multi_vault_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Check asset name length
        if name.len() > MAX_NAME_LEN {
            return Err(SolanaBridgeError::TokenNameLenLimit.into());
        }

        // Check asset symbol length
        if symbol.len() > MAX_SYMBOL_LEN {
            return Err(SolanaBridgeError::TokenSymbolLenLimit.into());
        }

        // If token settings account is not created
        if token_settings_account_info.lamports() == 0 {
            // Create Vault Account
            let (vault_pubkey, vault_nonce) = Pubkey::find_program_address(
                &[br"vault", &mint_account_info.key.to_bytes()],
                program_id,
            );
            let vault_account_signer_seeds: &[&[_]] =
                &[br"vault", &mint_account_info.key.to_bytes(), &[vault_nonce]];

            if vault_pubkey != *vault_account_info.key {
                return Err(ProgramError::InvalidArgument);
            }

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

            // Create Token Settings Account
            let (token_settings_pubkey, token_settings_nonce) = Pubkey::find_program_address(
                &[br"settings", &mint_account_info.key.to_bytes()],
                program_id,
            );
            let token_settings_account_signer_seeds: &[&[_]] = &[
                br"settings",
                &mint_account_info.key.to_bytes(),
                &[token_settings_nonce],
            ];

            if token_settings_pubkey != *token_settings_account_info.key {
                return Err(ProgramError::InvalidArgument);
            }

            invoke_signed(
                &system_instruction::create_account(
                    funder_account_info.key,
                    token_settings_account_info.key,
                    1.max(rent.minimum_balance(TokenSettings::LEN)),
                    TokenSettings::LEN as u64,
                    program_id,
                ),
                &[
                    funder_account_info.clone(),
                    token_settings_account_info.clone(),
                    system_program_info.clone(),
                ],
                &[token_settings_account_signer_seeds],
            )?;

            // Init Settings Account
            let token_settings_account_data = TokenSettings {
                is_initialized: true,
                account_kind: AccountKind::TokenSettings(token_settings_nonce, vault_nonce),
                kind: TokenKind::Solana {
                    mint: *mint_account_info.key,
                    vault: *vault_account_info.key,
                },
                name: name.clone(),
                symbol: symbol.clone(),
                withdrawal_epoch: 0,
                deposit_limit: u64::MAX,
                withdrawal_limit: u64::MAX,
                withdrawal_daily_limit: u64::MAX,
                withdrawal_daily_amount: 0,
                emergency: false,
                fee_info: Default::default(),
            };

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;
        }

        // Validate Token Settings Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (mint, vault) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
        let (token_settings_nonce, _vault_nonce) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_sol_account(
            program_id,
            &mint,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Mint Account
        if *mint_account_info.key != mint && mint_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        let mint_account_data = spl_token::state::Mint::unpack(&mint_account_info.data.borrow())?;
        let decimals = mint_account_data.decimals;

        // Validate Vault Account
        if *vault_account_info.key != vault && vault_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Multi Vault Account
        let multi_vault_account_data = MultiVault::unpack(&multi_vault_account_info.data.borrow())?;
        let multi_vault_nonce = multi_vault_account_data
            .account_kind
            .into_multi_vault()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_multi_vault_account(program_id, multi_vault_nonce, multi_vault_account_info)?;

        // Make transfer
        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data
            .amount
            .checked_add(amount)
            .ok_or(SolanaBridgeError::Overflow)?
            > token_settings_account_data.deposit_limit
        {
            return Err(SolanaBridgeError::DepositLimit.into());
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

        // Send sol amount to multi vault
        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                multi_vault_account_info.key,
                sol_amount,
            ),
            &[
                funder_account_info.clone(),
                multi_vault_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        // Create Deposit Account
        let (deposit_pubkey, deposit_nonce) =
            Pubkey::find_program_address(&[br"deposit", &deposit_seed.to_le_bytes()], program_id);
        let deposit_account_signer_seeds: &[&[_]] =
            &[br"deposit", &deposit_seed.to_le_bytes(), &[deposit_nonce]];

        if deposit_pubkey != *deposit_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                deposit_account_info.key,
                1.max(rent.minimum_balance(DepositMultiTokenSol::LEN)),
                DepositMultiTokenSol::LEN as u64,
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
        let fee_info = &mut token_settings_account_data.fee_info;

        let fee = 1.max(
            amount
                .checked_div(fee_info.divisor)
                .ok_or(SolanaBridgeError::Overflow)?
                .checked_mul(fee_info.multiplier)
                .ok_or(SolanaBridgeError::Overflow)?,
        );

        // Increase fee supply
        fee_info.supply = fee_info
            .supply
            .checked_add(fee)
            .ok_or(SolanaBridgeError::Overflow)?;

        // Amount without fee
        let pure_amount = amount.checked_sub(fee).ok_or(SolanaBridgeError::Overflow)?;

        // Amount in Ever decimals
        let transfer_amount = pure_amount as u128;

        let deposit_account_data = DepositMultiTokenSol {
            is_initialized: true,
            account_kind: AccountKind::Deposit(deposit_nonce),
            event: DepositMultiTokenSolEventWithLen::new(
                *mint_account_info.key,
                name,
                symbol,
                decimals,
                transfer_amount,
                sol_amount,
                recipient,
                payload,
            ),
            meta: DepositTokenMetaWithLen::new(deposit_seed),
        };

        DepositMultiTokenSol::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
        )?;

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_withdraw_multi_token_ever_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
        event_configuration: Pubkey,
        token: EverAddress,
        name: String,
        symbol: String,
        decimals: u8,
        recipient: Pubkey,
        amount: u128,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let author_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let rl_settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Check asset name length
        if name.len() > MAX_NAME_LEN {
            return Err(SolanaBridgeError::TokenNameLenLimit.into());
        }

        // Check asset symbol length
        if symbol.len() > MAX_SYMBOL_LEN {
            return Err(SolanaBridgeError::TokenSymbolLenLimit.into());
        }

        // Validate Round Loader Settings Account
        let rl_settings_account_data =
            round_loader::Settings::unpack(&rl_settings_account_info.data.borrow())?;

        let rl_settings_nonce = rl_settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            &round_loader::id(),
            rl_settings_nonce,
            rl_settings_account_info,
        )?;

        // Validate Relay Round Account
        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;
        let relay_round_nonce = relay_round_account_data
            .account_kind
            .into_relay_round()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let round_number = relay_round_account_data.round_number;

        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
            relay_round_nonce,
            relay_round_account_info,
        )?;

        if relay_round_account_data.round_end <= clock.unix_timestamp as u32 {
            return Err(SolanaBridgeError::RelayRoundExpired.into());
        }

        let mut required_votes = (relay_round_account_data.relays.len() * 2 / 3 + 1) as u32;
        if rl_settings_account_data.min_required_votes > required_votes {
            required_votes = rl_settings_account_data.min_required_votes;
        }

        let epoch = clock.unix_timestamp / SECONDS_PER_DAY as i64;

        // Create Withdraw Account
        let event = WithdrawalMultiTokenEverEventWithLen::new(
            token, name, symbol, decimals, amount, recipient,
        );

        let event_data = hash(&event.data.try_to_vec()?);

        let (withdrawal_pubkey, withdrawal_nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );
        let withdrawal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data.to_bytes(),
            &[withdrawal_nonce],
        ];

        if withdrawal_pubkey != *withdrawal_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                withdrawal_account_info.key,
                1.max(rent.minimum_balance(WithdrawalMultiTokenEver::LEN)),
                WithdrawalMultiTokenEver::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
            &[withdrawal_account_signer_seeds],
        )?;

        let withdrawal_account_data = WithdrawalMultiTokenEver {
            is_initialized: true,
            account_kind: AccountKind::Proposal(withdrawal_nonce),
            is_executed: false,
            author: *author_account_info.key,
            round_number,
            required_votes,
            event,
            pda: PDA {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
            },
            meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, epoch),
            signers: vec![Vote::None; relay_round_account_data.relays.len()],
        };

        WithdrawalMultiTokenEver::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        // Send voting reparation for Relay to withdrawal account
        let relays_lamports = RELAY_REPARATION * relay_round_account_data.relays.len() as u64;
        let token_settings_lamports = if token_settings_account_info.lamports() == 0 {
            1.max(rent.minimum_balance(spl_token::state::Mint::LEN))
                + 1.max(rent.minimum_balance(TokenSettings::LEN))
        } else {
            0
        };

        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                withdrawal_account_info.key,
                relays_lamports + token_settings_lamports,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        Ok(())
    }

    fn process_withdraw_multi_token_sol_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
        event_configuration: Pubkey,
        recipient: Pubkey,
        amount: u128,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let author_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let rl_settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Token Setting Account
        let token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (token_settings_nonce, _) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let (mint, _) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_sol_account(
            program_id,
            &mint,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        // Validate Round Loader Settings Account
        let rl_settings_account_data =
            round_loader::Settings::unpack(&rl_settings_account_info.data.borrow())?;

        let rl_settings_nonce = rl_settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            &round_loader::id(),
            rl_settings_nonce,
            rl_settings_account_info,
        )?;

        // Validate Relay Round Account
        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;
        let relay_round_nonce = relay_round_account_data
            .account_kind
            .into_relay_round()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let round_number = relay_round_account_data.round_number;

        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
            relay_round_nonce,
            relay_round_account_info,
        )?;

        if relay_round_account_data.round_end <= clock.unix_timestamp as u32 {
            return Err(SolanaBridgeError::RelayRoundExpired.into());
        }

        let mut required_votes = (relay_round_account_data.relays.len() * 2 / 3 + 1) as u32;
        if rl_settings_account_data.min_required_votes > required_votes {
            required_votes = rl_settings_account_data.min_required_votes;
        }

        let epoch = clock.unix_timestamp / SECONDS_PER_DAY as i64;

        // Create Withdraw Account
        let event = WithdrawalMultiTokenSolEventWithLen::new(mint, amount, recipient);

        let event_data = hash(&event.data.try_to_vec()?);

        let (withdrawal_pubkey, withdrawal_nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );
        let withdrawal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data.to_bytes(),
            &[withdrawal_nonce],
        ];

        if withdrawal_pubkey != *withdrawal_account_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                withdrawal_account_info.key,
                1.max(rent.minimum_balance(WithdrawalMultiTokenSol::LEN)),
                WithdrawalMultiTokenSol::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
            &[withdrawal_account_signer_seeds],
        )?;

        let withdrawal_account_data = WithdrawalMultiTokenSol {
            is_initialized: true,
            account_kind: AccountKind::Proposal(withdrawal_nonce),
            is_executed: false,
            author: *author_account_info.key,
            round_number,
            required_votes,
            pda: PDA {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
            },
            event,
            meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, epoch),
            signers: vec![Vote::None; relay_round_account_data.relays.len()],
        };

        WithdrawalMultiTokenSol::pack(
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

        // Validate vote
        if vote == Vote::None {
            return Err(SolanaBridgeError::InvalidVote.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            Proposal::unpack_from_slice(&withdrawal_account_info.data.borrow())?;

        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.try_to_vec()?[4..]);

        let (_, nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );

        bridge_utils::helper::validate_proposal_account(
            program_id,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            nonce,
            withdrawal_account_info,
        )?;

        // Check number of votes
        let sig_count = withdrawal_account_data
            .signers
            .iter()
            .filter(|vote| **vote == Vote::Confirm)
            .count() as u32;

        if sig_count == withdrawal_account_data.required_votes {
            return Err(SolanaBridgeError::VotesOverflow.into());
        }

        // Validate Relay Round Account
        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;
        let relay_round_nonce = relay_round_account_data
            .account_kind
            .into_relay_round()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        round_loader::validate_relay_round_account(
            &round_loader::id(),
            withdrawal_account_data.round_number,
            relay_round_nonce,
            relay_round_account_info,
        )?;

        // Vote for withdraw request
        let index = relay_round_account_data
            .relays
            .iter()
            .position(|pubkey| pubkey == relay_account_info.key)
            .ok_or(SolanaBridgeError::InvalidRelay)?;

        if withdrawal_account_data.signers[index] == Vote::None {
            // Vote for proposal
            withdrawal_account_data.signers[index] = vote;
            withdrawal_account_data.pack_into_slice(&mut withdrawal_account_info.data.borrow_mut());

            // Get back voting reparation to Relay
            let withdrawal_starting_lamports = withdrawal_account_info.lamports();
            **withdrawal_account_info.lamports.borrow_mut() = withdrawal_starting_lamports
                .checked_sub(RELAY_REPARATION)
                .ok_or(SolanaBridgeError::Overflow)?;

            let relay_starting_lamports = relay_account_info.lamports();
            **relay_account_info.lamports.borrow_mut() = relay_starting_lamports
                .checked_add(RELAY_REPARATION)
                .ok_or(SolanaBridgeError::Overflow)?;
        }

        Ok(())
    }

    fn process_withdraw_multi_token_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            WithdrawalMultiTokenEver::unpack(&withdrawal_account_info.data.borrow())?;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        let (_, withdrawal_nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );

        bridge_utils::helper::validate_proposal_account(
            program_id,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_nonce,
            withdrawal_account_info,
        )?;

        // If token settings account is not created
        if token_settings_account_info.lamports() == 0 {
            // Create Mint Account
            let ever_decimals = withdrawal_account_data.event.data.decimals;
            let solana_decimals = if ever_decimals > spl_token::native_mint::DECIMALS {
                spl_token::native_mint::DECIMALS
            } else {
                ever_decimals
            };
            let token = hash(&withdrawal_account_data.event.data.token.try_to_vec()?);

            let (mint_pubkey, mint_nonce) =
                Pubkey::find_program_address(&[br"mint", token.as_ref()], program_id);
            let mint_account_signer_seeds: &[&[_]] = &[br"mint", token.as_ref(), &[mint_nonce]];

            if mint_pubkey != *mint_account_info.key {
                return Err(ProgramError::InvalidArgument);
            }

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
                    solana_decimals,
                )?,
                &[
                    mint_account_info.clone(),
                    token_program_info.clone(),
                    rent_sysvar_info.clone(),
                ],
                &[mint_account_signer_seeds],
            )?;

            // Create Token Settings Account
            let (token_settings_pubkey, token_settings_nonce) =
                Pubkey::find_program_address(&[br"settings", token.as_ref()], program_id);
            let token_settings_account_signer_seeds: &[&[_]] =
                &[br"settings", token.as_ref(), &[token_settings_nonce]];

            if token_settings_pubkey != *token_settings_account_info.key {
                return Err(ProgramError::InvalidArgument);
            }

            invoke_signed(
                &system_instruction::create_account(
                    funder_account_info.key,
                    token_settings_account_info.key,
                    1.max(rent.minimum_balance(TokenSettings::LEN)),
                    TokenSettings::LEN as u64,
                    program_id,
                ),
                &[
                    funder_account_info.clone(),
                    token_settings_account_info.clone(),
                    system_program_info.clone(),
                ],
                &[token_settings_account_signer_seeds],
            )?;

            // Init Token Settings Account
            let token_settings_account_data = TokenSettings {
                is_initialized: true,
                account_kind: AccountKind::TokenSettings(token_settings_nonce, mint_nonce),
                kind: TokenKind::Ever {
                    mint: *mint_account_info.key,
                    decimals: ever_decimals,
                    token: withdrawal_account_data.event.data.token,
                },
                name: withdrawal_account_data.event.data.name.clone(),
                symbol: withdrawal_account_data.event.data.symbol.clone(),
                withdrawal_epoch: 0,
                deposit_limit: u64::MAX,
                withdrawal_limit: u64::MAX,
                withdrawal_daily_limit: u64::MAX,
                withdrawal_daily_amount: 0,
                emergency: false,
                fee_info: Default::default(),
            };

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;

            // Get back reparation for creating accounts to funder
            let reparation = 1.max(rent.minimum_balance(TokenSettings::LEN))
                + 1.max(rent.minimum_balance(spl_token::state::Mint::LEN));

            let withdrawal_starting_lamports = withdrawal_account_info.lamports();
            **withdrawal_account_info.lamports.borrow_mut() = withdrawal_starting_lamports
                .checked_sub(reparation)
                .ok_or(SolanaBridgeError::Overflow)?;

            let relay_starting_lamports = funder_account_info.lamports();
            **funder_account_info.lamports.borrow_mut() = relay_starting_lamports
                .checked_add(reparation)
                .ok_or(SolanaBridgeError::Overflow)?;
        }

        // Validate Token Setting Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (token_settings_nonce, mint_nonce) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let (_, token, _) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Mint Account
        validate_mint_account(program_id, &token, mint_nonce, mint_account_info)?;

        // Check connection between token and proposal
        if token != withdrawal_account_data.event.data.token {
            return Err(ProgramError::InvalidArgument);
        }

        let mint_account_data = spl_token::state::Mint::unpack(&mint_account_info.data.borrow())?;
        let solana_decimals = mint_account_data.decimals;
        let ever_decimals = withdrawal_account_data.event.data.decimals;

        // Do we have enough signers.
        let sig_count = withdrawal_account_data
            .signers
            .iter()
            .filter(|vote| **vote == Vote::Confirm)
            .count() as u32;

        if sig_count == withdrawal_account_data.required_votes
            && withdrawal_account_data.meta.data.status == WithdrawalTokenStatus::New
        {
            let current_epoch = clock.unix_timestamp / SECONDS_PER_DAY as i64;

            // If current epoch has changed
            if token_settings_account_data.withdrawal_epoch != current_epoch {
                token_settings_account_data.withdrawal_epoch = current_epoch;
                token_settings_account_data.withdrawal_daily_amount = Default::default();
            }

            // Calculate amount
            let withdrawal_amount = get_withdrawal_amount(
                withdrawal_account_data.event.data.amount,
                ever_decimals,
                solana_decimals,
            );

            let fee_info = &mut token_settings_account_data.fee_info;

            let fee = 1.max(
                withdrawal_amount
                    .checked_div(fee_info.divisor)
                    .ok_or(SolanaBridgeError::Overflow)?
                    .checked_mul(fee_info.multiplier)
                    .ok_or(SolanaBridgeError::Overflow)?,
            );

            // Increase fee supply
            fee_info.supply = fee_info
                .supply
                .checked_add(fee)
                .ok_or(SolanaBridgeError::Overflow)?;

            // Amount without fee
            let transfer_withdrawal_amount = withdrawal_amount
                .checked_sub(fee)
                .ok_or(SolanaBridgeError::Overflow)?;

            // Increase withdrawal daily amount
            token_settings_account_data.withdrawal_daily_amount = token_settings_account_data
                .withdrawal_daily_amount
                .checked_add(transfer_withdrawal_amount)
                .ok_or(SolanaBridgeError::Overflow)?;

            if transfer_withdrawal_amount > token_settings_account_data.withdrawal_limit
                || token_settings_account_data.withdrawal_daily_amount
                    > token_settings_account_data.withdrawal_daily_limit
            {
                withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::WaitingForApprove;
            } else {
                make_multi_token_ever_transfer(
                    mint_account_info,
                    token_program_info,
                    recipient_token_account_info,
                    &token_settings_account_data,
                    &mut withdrawal_account_data,
                    transfer_withdrawal_amount,
                )?;
            }

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;

            withdrawal_account_data.is_executed = true;

            WithdrawalMultiTokenEver::pack(
                withdrawal_account_data,
                &mut withdrawal_account_info.data.borrow_mut(),
            )?;
        }

        Ok(())
    }

    fn process_withdraw_multi_token_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            WithdrawalMultiTokenSol::unpack(&withdrawal_account_info.data.borrow())?;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        let (_, withdrawal_nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );

        bridge_utils::helper::validate_proposal_account(
            program_id,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_nonce,
            withdrawal_account_info,
        )?;

        let withdrawal_status = withdrawal_account_data.meta.data.status;

        // Validate Token Setting Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (token_settings_nonce, vault_nonce) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let (mint, _) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_sol_account(
            program_id,
            &mint,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Vault Account
        validate_vault_account(program_id, &mint, vault_nonce, vault_account_info)?;

        // Check connection between token and proposal
        if mint != withdrawal_account_data.event.data.mint {
            return Err(ProgramError::InvalidArgument);
        }

        // Do we have enough signers.
        let sig_count = withdrawal_account_data
            .signers
            .iter()
            .filter(|vote| **vote == Vote::Confirm)
            .count() as u32;

        if sig_count == withdrawal_account_data.required_votes {
            let withdrawal_amount = withdrawal_account_data.event.data.amount as u64;

            let fee_info = &mut token_settings_account_data.fee_info;

            let fee = 1.max(
                withdrawal_amount
                    .checked_div(fee_info.divisor)
                    .ok_or(SolanaBridgeError::Overflow)?
                    .checked_mul(fee_info.multiplier)
                    .ok_or(SolanaBridgeError::Overflow)?,
            );

            // Amount without fee
            let transfer_withdrawal_amount = withdrawal_amount
                .checked_sub(fee)
                .ok_or(SolanaBridgeError::Overflow)?;

            match withdrawal_status {
                WithdrawalTokenStatus::New => {
                    let current_epoch = clock.unix_timestamp / SECONDS_PER_DAY as i64;

                    // If current epoch has changed
                    if token_settings_account_data.withdrawal_epoch != current_epoch {
                        token_settings_account_data.withdrawal_epoch = current_epoch;
                        token_settings_account_data.withdrawal_daily_amount = Default::default();
                    }

                    // Increase withdrawal daily amount
                    token_settings_account_data.withdrawal_daily_amount =
                        token_settings_account_data
                            .withdrawal_daily_amount
                            .checked_add(transfer_withdrawal_amount)
                            .ok_or(SolanaBridgeError::Overflow)?;

                    // Increase fee supply
                    fee_info.supply = fee_info
                        .supply
                        .checked_add(fee)
                        .ok_or(SolanaBridgeError::Overflow)?;

                    if transfer_withdrawal_amount > token_settings_account_data.withdrawal_limit
                        || token_settings_account_data.withdrawal_daily_amount
                            > token_settings_account_data.withdrawal_daily_limit
                    {
                        withdrawal_account_data.meta.data.status =
                            WithdrawalTokenStatus::WaitingForApprove;
                    } else {
                        make_multi_token_sol_transfer(
                            vault_account_info,
                            token_program_info,
                            recipient_token_account_info,
                            &token_settings_account_data,
                            &mut withdrawal_account_data,
                            transfer_withdrawal_amount,
                        )?;
                    }

                    TokenSettings::pack(
                        token_settings_account_data,
                        &mut token_settings_account_info.data.borrow_mut(),
                    )?;

                    withdrawal_account_data.is_executed = true
                }
                WithdrawalTokenStatus::Pending => make_multi_token_sol_transfer(
                    vault_account_info,
                    token_program_info,
                    recipient_token_account_info,
                    &token_settings_account_data,
                    &mut withdrawal_account_data,
                    transfer_withdrawal_amount,
                )?,
                _ => (),
            }
        }

        WithdrawalMultiTokenSol::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_guardian(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_guardian: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Initializer Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        settings_account_data.guardian = new_guardian;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_manager(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_manager: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Initializer Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        settings_account_data.manager = new_manager;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_withdrawal_manager(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_withdrawal_manager: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Initializer Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        settings_account_data.withdrawal_manager = new_withdrawal_manager;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_deposit_limit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_deposit_limit: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        // Validate Initializer Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Token Settings Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (_, token, _) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
        let (token_settings_nonce, _) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        token_settings_account_data.deposit_limit = new_deposit_limit;

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_change_withdrawal_limits(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_withdrawal_limit: Option<u64>,
        new_withdrawal_daily_limit: Option<u64>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        // Validate Initializer Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Token Settings Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (_, token, _) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
        let (token_settings_nonce, _) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if let Some(new_withdrawal_limit) = new_withdrawal_limit {
            token_settings_account_data.withdrawal_limit = new_withdrawal_limit;
        }

        if let Some(new_withdrawal_daily_limit) = new_withdrawal_daily_limit {
            token_settings_account_data.withdrawal_daily_limit = new_withdrawal_daily_limit;
        }

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_enable_emergency_mode(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        let guardian = settings_account_data.guardian;

        // Validate Guardian Account
        if *authority_account_info.key != guardian {
            let programdata_account_info = next_account_info(account_info_iter)?;

            // Validate Initializer Account
            bridge_utils::helper::validate_programdata_account(
                program_id,
                programdata_account_info.key,
            )?;
            bridge_utils::helper::validate_initializer_account(
                authority_account_info.key,
                programdata_account_info,
            )?;
        }

        settings_account_data.emergency = true;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_disable_emergency_mode(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Owner Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        settings_account_data.emergency = false;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_enable_token_emergency_mode(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        let guardian = settings_account_data.guardian;

        // Validate Guardian Account
        if *authority_account_info.key != guardian {
            let programdata_account_info = next_account_info(account_info_iter)?;

            // Validate Initializer Account
            bridge_utils::helper::validate_programdata_account(
                program_id,
                programdata_account_info.key,
            )?;
            bridge_utils::helper::validate_initializer_account(
                authority_account_info.key,
                programdata_account_info,
            )?;
        }

        // Validate Token Settings Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (_, token, _) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
        let (token_settings_nonce, _) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        token_settings_account_data.emergency = true;

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_disable_token_emergency_mode(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Owner Account
        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            authority_account_info.key,
            programdata_account_info,
        )?;

        // Validate Token Settings Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (_, token, _) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
        let (token_settings_nonce, _) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        token_settings_account_data.emergency = false;

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_approve_withdraw_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let authority_account_info = next_account_info(account_info_iter)?;
        let mint_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            WithdrawalMultiTokenEver::unpack(&withdrawal_account_info.data.borrow())?;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        let (_, withdrawal_nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );

        bridge_utils::helper::validate_proposal_account(
            program_id,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_nonce,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::WaitingForApprove {
            return Err(SolanaBridgeError::InvalidWithdrawalStatus.into());
        }

        // Validate Token Setting Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (token_settings_nonce, mint_nonce) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let (_, token, _) = token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_ever_account(
            program_id,
            &token,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Mint Account
        validate_mint_account(program_id, &token, mint_nonce, mint_account_info)?;

        // Check connection between token and proposal
        if token != withdrawal_account_data.event.data.token {
            return Err(ProgramError::InvalidArgument);
        }

        if *authority_account_info.key != settings_account_data.withdrawal_manager {
            let programdata_account_info = next_account_info(account_info_iter)?;

            // Validate Initializer Account
            bridge_utils::helper::validate_programdata_account(
                program_id,
                programdata_account_info.key,
            )?;
            bridge_utils::helper::validate_initializer_account(
                authority_account_info.key,
                programdata_account_info,
            )?;
        }

        let mint_account_data = spl_token::state::Mint::unpack(&mint_account_info.data.borrow())?;
        let solana_decimals = mint_account_data.decimals;
        let ever_decimals = withdrawal_account_data.event.data.decimals;

        let withdrawal_amount = get_withdrawal_amount(
            withdrawal_account_data.event.data.amount,
            ever_decimals,
            solana_decimals,
        );

        make_multi_token_ever_transfer(
            mint_account_info,
            token_program_info,
            recipient_token_account_info,
            &token_settings_account_data,
            &mut withdrawal_account_data,
            withdrawal_amount,
        )?;

        let current_epoch = clock.unix_timestamp / SECONDS_PER_DAY as i64;

        // If withdrawal is in current epoch
        if withdrawal_account_data.meta.data.epoch == current_epoch {
            // Decrease withdrawal daily amount
            token_settings_account_data.withdrawal_daily_amount = token_settings_account_data
                .withdrawal_daily_amount
                .checked_sub(withdrawal_amount)
                .ok_or(SolanaBridgeError::Overflow)?;

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;
        }

        WithdrawalMultiTokenEver::pack(
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

        let authority_account_info = next_account_info(account_info_iter)?;
        let vault_account_info = next_account_info(account_info_iter)?;
        let withdrawal_account_info = next_account_info(account_info_iter)?;
        let recipient_token_account_info = next_account_info(account_info_iter)?;
        let token_settings_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;

        let token_program_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let settings_nonce = settings_account_data
            .account_kind
            .into_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        bridge_utils::helper::validate_settings_account(
            program_id,
            settings_nonce,
            settings_account_info,
        )?;

        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            WithdrawalMultiTokenSol::unpack(&withdrawal_account_info.data.borrow())?;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        let (_, withdrawal_nonce) = Pubkey::find_program_address(
            &[
                br"proposal",
                &round_number.to_le_bytes(),
                &event_timestamp.to_le_bytes(),
                &event_transaction_lt.to_le_bytes(),
                &event_configuration.to_bytes(),
                &event_data.to_bytes(),
            ],
            program_id,
        );

        bridge_utils::helper::validate_proposal_account(
            program_id,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_nonce,
            withdrawal_account_info,
        )?;

        if withdrawal_account_data.meta.data.status != WithdrawalTokenStatus::WaitingForApprove {
            return Err(SolanaBridgeError::InvalidWithdrawalStatus.into());
        }

        if *authority_account_info.key != settings_account_data.withdrawal_manager {
            let programdata_account_info = next_account_info(account_info_iter)?;

            // Validate Initializer Account
            bridge_utils::helper::validate_programdata_account(
                program_id,
                programdata_account_info.key,
            )?;
            bridge_utils::helper::validate_initializer_account(
                authority_account_info.key,
                programdata_account_info,
            )?;
        }

        // Validate Token Setting Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let (token_settings_nonce, vault_nonce) = token_settings_account_data
            .account_kind
            .into_token_settings()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        let (mint, _) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_sol_account(
            program_id,
            &mint,
            token_settings_nonce,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Vault Account
        validate_vault_account(program_id, &mint, vault_nonce, vault_account_info)?;

        // Check connection between token and proposal
        if mint != withdrawal_account_data.event.data.mint {
            return Err(ProgramError::InvalidArgument);
        }

        let withdrawal_amount = withdrawal_account_data.event.data.amount as u64;

        make_multi_token_sol_transfer(
            vault_account_info,
            token_program_info,
            recipient_token_account_info,
            &token_settings_account_data,
            &mut withdrawal_account_data,
            withdrawal_amount,
        )?;

        let current_epoch = clock.unix_timestamp / SECONDS_PER_DAY as i64;

        // If withdrawal is in current epoch
        if withdrawal_account_data.meta.data.epoch == current_epoch {
            // Decrease withdrawal daily amount
            token_settings_account_data.withdrawal_daily_amount = token_settings_account_data
                .withdrawal_daily_amount
                .checked_sub(withdrawal_amount)
                .ok_or(SolanaBridgeError::Overflow)?;

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;
        }

        WithdrawalMultiTokenSol::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }
}

fn make_multi_token_sol_transfer<'a>(
    vault_account_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    recipient_token_account_info: &AccountInfo<'a>,
    settings_account_data: &TokenSettings,
    withdrawal_account_data: &mut WithdrawalMultiTokenSol,
    withdrawal_amount: u64,
) -> ProgramResult {
    // Validate Recipient Account
    if recipient_token_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    let recipient_token_account_data =
        spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

    if recipient_token_account_data.owner != withdrawal_account_data.event.data.recipient {
        return Err(ProgramError::InvalidArgument);
    }

    // Transfer tokens from Vault Account to Recipient Account
    let (mint, _) = settings_account_data
        .kind
        .into_solana()
        .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

    let (_, vault_nonce) = settings_account_data
        .account_kind
        .into_token_settings()
        .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

    let vault_account_signer_seeds: &[&[_]] = &[br"vault", &mint.to_bytes(), &[vault_nonce]];

    if vault_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    let vault_account_data = spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

    // If vault balance is not enough
    if withdrawal_amount > vault_account_data.amount {
        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;
        return Ok(());
    }

    invoke_signed(
        &spl_token::instruction::transfer(
            &spl_token::id(),
            vault_account_info.key,
            recipient_token_account_info.key,
            vault_account_info.key,
            &[vault_account_info.key],
            withdrawal_amount,
        )?,
        &[
            token_program_info.clone(),
            vault_account_info.clone(),
            recipient_token_account_info.clone(),
        ],
        &[vault_account_signer_seeds],
    )?;

    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;

    Ok(())
}

fn make_multi_token_ever_transfer<'a>(
    mint_account_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    recipient_token_account_info: &AccountInfo<'a>,
    settings_account_data: &TokenSettings,
    withdrawal_account_data: &mut WithdrawalMultiTokenEver,
    withdrawal_amount: u64,
) -> ProgramResult {
    // Validate Recipient Account
    if recipient_token_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    let recipient_token_account_data =
        spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

    if recipient_token_account_data.owner != withdrawal_account_data.event.data.recipient {
        return Err(ProgramError::InvalidArgument);
    }

    // Mint EVER tokens to Recipient Account
    let (_, token, _) = settings_account_data
        .kind
        .into_ever()
        .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;
    let (_, mint_nonce) = settings_account_data
        .account_kind
        .into_token_settings()
        .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

    let token_hash = hash(&token.try_to_vec()?);
    let mint_account_signer_seeds: &[&[_]] = &[br"mint", token_hash.as_ref(), &[mint_nonce]];

    if mint_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    invoke_signed(
        &spl_token::instruction::mint_to(
            &spl_token::id(),
            mint_account_info.key,
            recipient_token_account_info.key,
            mint_account_info.key,
            &[mint_account_info.key],
            withdrawal_amount,
        )?,
        &[
            token_program_info.clone(),
            mint_account_info.clone(),
            recipient_token_account_info.clone(),
        ],
        &[mint_account_signer_seeds],
    )?;

    withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Processed;

    Ok(())
}

pub fn get_withdrawal_amount(amount: u128, ever_decimals: u8, solana_decimals: u8) -> u64 {
    if ever_decimals > solana_decimals {
        let trunc_divisor = 10u128.pow((ever_decimals - solana_decimals) as u32);
        (amount / trunc_divisor) as u64
    } else {
        let trunc_divisor = 10u128.pow((solana_decimals - ever_decimals) as u32);
        (amount * trunc_divisor) as u64
    }
}

pub fn get_deposit_amount(amount: u64, ever_decimals: u8, solana_decimals: u8) -> u128 {
    let mut amount = amount as u128;
    if ever_decimals > solana_decimals {
        let trunc_divisor = 10u128.pow((ever_decimals - solana_decimals) as u32);
        amount *= trunc_divisor;
    } else {
        let trunc_divisor = 10u128.pow((solana_decimals - ever_decimals) as u32);
        amount /= trunc_divisor;
    }
    amount
}
