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
                withdrawal_manager,
            } => {
                msg!("Instruction: Initialize Token Proxy");
                Self::process_initialize(program_id, accounts, guardian, withdrawal_manager)?;
            }
            TokenProxyInstruction::DepositMultiTokenEver {
                deposit_seed,
                recipient_address,
                token_address,
                amount,
                sol_amount,
                payload,
            } => {
                msg!("Instruction: Deposit MULTI TOKEN EVER");
                Self::process_deposit_multi_token_ever(
                    program_id,
                    accounts,
                    deposit_seed,
                    recipient_address,
                    token_address,
                    amount,
                    sol_amount,
                    payload,
                )?;
            }
            TokenProxyInstruction::DepositMultiTokenSol {
                deposit_seed,
                recipient_address,
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
                    recipient_address,
                    amount,
                    name,
                    symbol,
                    sol_amount,
                    payload,
                )?;
            }
            TokenProxyInstruction::WithdrawMultiTokenEverRequest {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
                token_address,
                name,
                symbol,
                decimals,
                recipient_address,
                amount,
            } => {
                msg!("Instruction: Withdraw Multi token EVER request");
                Self::process_withdraw_multi_token_ever_request(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    event_configuration,
                    token_address,
                    name,
                    symbol,
                    decimals,
                    recipient_address,
                    amount,
                )?;
            }
            TokenProxyInstruction::WithdrawMultiTokenSolRequest {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
                token_address,
                recipient_address,
                amount,
            } => {
                msg!("Instruction: Withdraw multi token SOL request");
                Self::process_withdraw_multi_token_sol_request(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    event_configuration,
                    token_address,
                    recipient_address,
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
        };

        Ok(())
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        guardian: Pubkey,
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

        // Validate Settings Account
        let settings_nonce =
            bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;
        let settings_account_signer_seeds: &[&[_]] = &[br"settings", &[settings_nonce]];

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
            guardian,
            withdrawal_manager,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Validate Multi Vault Account
        let multi_vault_nonce = validate_multi_vault_account(program_id, multi_vault_account_info)?;
        let multi_vault_account_signer_seeds: &[&[_]] = &[br"multivault", &[multi_vault_nonce]];

        // Create Multi Vault Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                multi_vault_account_info.key,
                1.max(rent.minimum_balance(0)),
                0,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                multi_vault_account_info.clone(),
                system_program_info.clone(),
            ],
            &[multi_vault_account_signer_seeds],
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_deposit_multi_token_ever(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        recipient_address: EverAddress,
        token_address: EverAddress,
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
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Token Settings Account
        let token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let name = &token_settings_account_data.name;
        let symbol = &token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let mint = &token_settings_account_data
            .kind
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            mint,
            token_settings_account_info,
        )?;

        if let TokenKind::Ever { mint, .. } = token_settings_account_data.kind {
            if *mint_account_info.key != mint {
                return Err(SolanaBridgeError::InvalidTokenKind.into());
            }
        } else {
            return Err(SolanaBridgeError::InvalidTokenKind.into());
        }

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Mint Account
        validate_mint_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            mint_account_info,
        )?;

        // Validate Multi Vault Account
        validate_multi_vault_account(program_id, multi_vault_account_info)?;

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
            token_settings_account_info.key,
            deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &token_settings_account_info.key.to_bytes(),
            &[deposit_nonce],
        ];

        // Create Deposit Account
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

        let amount = get_deposit_amount(
            amount,
            token_settings_account_data.ever_decimals,
            token_settings_account_data.solana_decimals,
        );

        // Init Deposit Account
        let deposit_account_data = DepositMultiTokenEver {
            is_initialized: true,
            account_kind: AccountKind::Deposit,
            event: DepositMultiTokenEverEventWithLen::new(
                token_address,
                amount,
                sol_amount,
                recipient_address,
                payload,
            ),
            meta: DepositTokenMetaWithLen::new(deposit_seed),
        };

        DepositMultiTokenEver::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_deposit_multi_token_sol(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        deposit_seed: u128,
        recipient_address: EverAddress,
        amount: u64,
        name: String,
        symbol: String,
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
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Check asset name and symbol length
        if name.len() > MAX_NAME_LEN || symbol.len() > MAX_NAME_LEN {
            return Err(SolanaBridgeError::TokenNameLenLimit.into());
        }

        // If token settings account is not created
        if token_settings_account_info.lamports() == 0 {
            let mint_account_data =
                spl_token::state::Mint::unpack(&mint_account_info.data.borrow())?;
            let solana_decimals = mint_account_data.decimals;
            let ever_decimals = mint_account_data.decimals;

            // Validate Vault Account
            let vault_nonce = validate_vault_account(
                program_id,
                &name,
                &symbol,
                ever_decimals,
                solana_decimals,
                mint_account_info.key,
                vault_account_info,
            )?;
            let vault_account_signer_seeds: &[&[_]] = &[
                br"vault",
                name.as_bytes(),
                symbol.as_bytes(),
                &ever_decimals.to_le_bytes(),
                &solana_decimals.to_le_bytes(),
                &mint_account_info.key.to_bytes(),
                &[vault_nonce],
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
            let token_settings_nonce = validate_token_settings_account(
                program_id,
                &name,
                &symbol,
                ever_decimals,
                solana_decimals,
                mint_account_info.key,
                token_settings_account_info,
            )?;
            let token_settings_account_signer_seeds: &[&[_]] = &[
                br"settings",
                name.as_bytes(),
                symbol.as_bytes(),
                &ever_decimals.to_le_bytes(),
                &solana_decimals.to_le_bytes(),
                &mint_account_info.key.to_bytes(),
                &[token_settings_nonce],
            ];

            // Create Settings Account
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
                account_kind: AccountKind::Settings,
                kind: TokenKind::Solana {
                    mint: *mint_account_info.key,
                    vault: *vault_account_info.key,
                },
                withdrawal_daily_amount: 0,
                withdrawal_epoch: 0,
                name: name.clone(),
                symbol: symbol.clone(),
                ever_decimals,
                solana_decimals,
                deposit_limit: 0,
                withdrawal_limit: 0,
                withdrawal_daily_limit: 0,
                emergency: false,
            };

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;
        }

        // Validate Token Setting Account
        let token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let name = token_settings_account_data.name;
        let symbol = token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let (mint_account, vault_account) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_account(
            program_id,
            &name,
            &symbol,
            ever_decimals,
            solana_decimals,
            &mint_account,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Multi Vault Account
        validate_multi_vault_account(program_id, multi_vault_account_info)?;

        // Validate Mint Account
        if *mint_account_info.key != mint_account && mint_account_info.owner != &spl_token::id() {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate Vault Account
        if *vault_account_info.key != vault_account && vault_account_info.owner != &spl_token::id()
        {
            return Err(ProgramError::InvalidArgument);
        }

        let vault_account_data =
            spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

        if vault_account_data
            .amount
            .checked_add(amount)
            .ok_or(SolanaBridgeError::Overflow)?
            > token_settings_account_data.deposit_limit
            && token_settings_account_data.deposit_limit != 0
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

        // Validate Deposit account
        let deposit_nonce = validate_deposit_account(
            program_id,
            deposit_seed,
            token_settings_account_info.key,
            deposit_account_info,
        )?;
        let deposit_account_signer_seeds: &[&[_]] = &[
            br"deposit",
            &deposit_seed.to_le_bytes(),
            &token_settings_account_info.key.to_bytes(),
            &[deposit_nonce],
        ];

        // Create Deposit Account
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

        let amount = get_deposit_amount(
            amount,
            token_settings_account_data.ever_decimals,
            token_settings_account_data.solana_decimals,
        );

        // Init Deposit Account
        let deposit_account_data = DepositMultiTokenSol {
            is_initialized: true,
            account_kind: AccountKind::Deposit,
            event: DepositMultiTokenSolEventWithLen::new(
                *mint_account_info.key,
                name,
                symbol,
                token_settings_account_data.solana_decimals,
                amount,
                sol_amount,
                recipient_address,
                payload,
            ),
            meta: DepositTokenMetaWithLen::new(deposit_seed),
        };

        DepositMultiTokenSol::pack(
            deposit_account_data,
            &mut deposit_account_info.data.borrow_mut(),
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
        token_address: EverAddress,
        name: String,
        symbol: String,
        decimals: u8,
        recipient_address: Pubkey,
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

        // Check asset name and symbol lengths
        if name.len() > MAX_NAME_LEN || symbol.len() > MAX_NAME_LEN {
            return Err(SolanaBridgeError::TokenNameLenLimit.into());
        }

        // Validate Round Loader Settings Account
        bridge_utils::helper::validate_settings_account(
            &round_loader::id(),
            rl_settings_account_info,
        )?;

        let rl_settings_account_data =
            round_loader::Settings::unpack(&rl_settings_account_info.data.borrow())?;

        // Validate Relay Round Account
        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;
        let round_number = relay_round_account_data.round_number;

        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
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

        // Init Withdraw Account
        let withdrawal_account_data = WithdrawalMultiTokenEver {
            is_initialized: true,
            account_kind: AccountKind::Proposal,
            is_executed: false,
            author: *author_account_info.key,
            round_number,
            required_votes,
            event: WithdrawalMultiTokenEverEventWithLen::new(
                token_address,
                name,
                symbol,
                decimals,
                amount,
                recipient_address,
            ),
            meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, epoch),
            signers: vec![Vote::None; relay_round_account_data.relays.len()],
            pda: PDA {
                settings: *token_settings_account_info.key,
                event_timestamp,
                event_transaction_lt,
                event_configuration,
            },
        };

        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        // Validate Withdrawal Account
        let withdrawal_nonce = bridge_utils::helper::validate_proposal_account(
            program_id,
            token_settings_account_info.key,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_account_info,
        )?;
        let withdrawal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &token_settings_account_info.key.to_bytes(),
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data.to_bytes(),
            &[withdrawal_nonce],
        ];

        // Create Withdraw Account
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

        WithdrawalMultiTokenEver::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

        let create_token_settings_lamports = if token_settings_account_info.lamports() == 0 {
            1.max(rent.minimum_balance(spl_token::state::Mint::LEN))
                + 1.max(rent.minimum_balance(TokenSettings::LEN))
                + RELAY_REPARATION
        } else {
            0
        };

        // Send voting reparation for Relay to withdrawal account
        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                withdrawal_account_info.key,
                (RELAY_REPARATION * relay_round_account_data.relays.len() as u64)
                    + create_token_settings_lamports,
            ),
            &[
                funder_account_info.clone(),
                withdrawal_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_withdraw_multi_token_sol_request(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
        event_configuration: Pubkey,
        token_address: Pubkey,
        recipient_address: Pubkey,
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

        // Validate Setting Account
        let token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let name = &token_settings_account_data.name;
        let symbol = &token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let (mint, _) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
            &token_settings_account_info,
        )?;

        // Validate Round Loader Settings Account
        bridge_utils::helper::validate_settings_account(
            &round_loader::id(),
            rl_settings_account_info,
        )?;

        let rl_settings_account_data =
            round_loader::Settings::unpack(&rl_settings_account_info.data.borrow())?;

        // Validate Relay Round Account
        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;
        let round_number = relay_round_account_data.round_number;

        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
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

        // Init Withdraw Account
        let withdrawal_account_data = WithdrawalMultiTokenSol {
            is_initialized: true,
            account_kind: AccountKind::Proposal,
            is_executed: false,
            author: *author_account_info.key,
            round_number,
            required_votes,
            event: WithdrawalMultiTokenSolEventWithLen::new(
                token_address,
                amount,
                recipient_address,
            ),
            meta: WithdrawalTokenMetaWithLen::new(WithdrawalTokenStatus::New, 0, epoch),
            signers: vec![Vote::None; relay_round_account_data.relays.len()],
            pda: PDA {
                settings: *token_settings_account_info.key,
                event_timestamp,
                event_transaction_lt,
                event_configuration,
            },
        };

        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        // Validate Withdrawal Account
        let withdrawal_nonce = bridge_utils::helper::validate_proposal_account(
            program_id,
            token_settings_account_info.key,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_account_info,
        )?;
        let withdrawal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &token_settings_account_info.key.to_bytes(),
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data.to_bytes(),
            &[withdrawal_nonce],
        ];

        // Create Withdraw Account
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

        let settings = withdrawal_account_data.pda.settings;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.try_to_vec()?[4..]);

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &settings,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_account_info,
        )?;

        let round_number = withdrawal_account_data.round_number;

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
        round_loader::validate_relay_round_account(
            &round_loader::id(),
            round_number,
            relay_round_account_info,
        )?;

        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;

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
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            WithdrawalMultiTokenEver::unpack(&withdrawal_account_info.data.borrow())?;
        let settings = withdrawal_account_data.pda.settings;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        let name = withdrawal_account_data.event.data.name.clone();
        let symbol = withdrawal_account_data.event.data.symbol.clone();
        let decimals = withdrawal_account_data.event.data.decimals;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &settings,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_account_info,
        )?;

        // Validate Token Setting Account
        if token_settings_account_info.lamports() == 0 {
            // Validate Mint Account
            let mint_nonce = validate_mint_account(
                program_id,
                &name,
                &symbol,
                decimals,
                decimals,
                mint_account_info,
            )?;
            let mint_account_signer_seeds: &[&[_]] = &[
                br"mint",
                name.as_bytes(),
                symbol.as_bytes(),
                &decimals.to_le_bytes(),
                &decimals.to_le_bytes(),
                &[mint_nonce],
            ];

            // Create Mint Account
            invoke_signed(
                &system_instruction::create_account(
                    withdrawal_account_info.key,
                    mint_account_info.key,
                    1.max(rent.minimum_balance(spl_token::state::Mint::LEN)),
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                &[
                    withdrawal_account_info.clone(),
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
                    token_program_info.clone(),
                    rent_sysvar_info.clone(),
                ],
                &[mint_account_signer_seeds],
            )?;

            // Validate Settings Account
            let token_settings_nonce = validate_token_settings_account(
                program_id,
                &name,
                &symbol,
                decimals,
                decimals,
                mint_account_info.key,
                token_settings_account_info,
            )?;
            let token_settings_account_signer_seeds: &[&[_]] =
                &[br"settings", name.as_bytes(), &[token_settings_nonce]];

            // Create Settings Account
            invoke_signed(
                &system_instruction::create_account(
                    withdrawal_account_info.key,
                    token_settings_account_info.key,
                    1.max(rent.minimum_balance(TokenSettings::LEN)),
                    TokenSettings::LEN as u64,
                    program_id,
                ),
                &[
                    withdrawal_account_info.clone(),
                    token_settings_account_info.clone(),
                    system_program_info.clone(),
                ],
                &[token_settings_account_signer_seeds],
            )?;

            // Init Settings Account
            let token_settings_account_data = TokenSettings {
                is_initialized: true,
                account_kind: AccountKind::Settings,
                kind: TokenKind::Ever {
                    mint: *mint_account_info.key,
                },
                withdrawal_daily_amount: 0,
                withdrawal_epoch: 0,
                name: name.clone(),
                symbol: symbol.clone(),
                ever_decimals: decimals,
                solana_decimals: decimals,
                deposit_limit: 0,
                withdrawal_limit: 0,
                withdrawal_daily_limit: 0,
                emergency: false,
            };

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;
        }

        // Validate Setting Account
        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let name = &token_settings_account_data.name;
        let symbol = &token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let mint = token_settings_account_data
            .kind
            .clone()
            .into_ever()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
            token_settings_account_info,
        )?;

        if *token_settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        let withdrawal_amount = get_withdrawal_amount(
            withdrawal_account_data.event.data.amount,
            token_settings_account_data.ever_decimals,
            token_settings_account_data.solana_decimals,
        );

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

            // Increase withdrawal daily amount
            token_settings_account_data.withdrawal_daily_amount = token_settings_account_data
                .withdrawal_daily_amount
                .checked_add(withdrawal_amount)
                .ok_or(SolanaBridgeError::Overflow)?;

            if withdrawal_amount > token_settings_account_data.withdrawal_limit
                || token_settings_account_data.withdrawal_daily_amount
                    > token_settings_account_data.withdrawal_daily_limit
            {
                withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::WaitingForApprove;
            } else {
                make_multi_token_ever_transfer(
                    program_id,
                    mint_account_info,
                    token_program_info,
                    recipient_token_account_info,
                    &token_settings_account_data,
                    &mut withdrawal_account_data,
                    withdrawal_amount,
                )?;
            }

            TokenSettings::pack(
                token_settings_account_data,
                &mut token_settings_account_info.data.borrow_mut(),
            )?;

            withdrawal_account_data.is_executed = true;
        }

        WithdrawalMultiTokenEver::pack(
            withdrawal_account_data,
            &mut withdrawal_account_info.data.borrow_mut(),
        )?;

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
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        // Validate Withdrawal Account
        let mut withdrawal_account_data =
            WithdrawalMultiTokenSol::unpack(&withdrawal_account_info.data.borrow())?;
        let settings = withdrawal_account_data.pda.settings;
        let round_number = withdrawal_account_data.round_number;
        let event_timestamp = withdrawal_account_data.pda.event_timestamp;
        let event_transaction_lt = withdrawal_account_data.pda.event_transaction_lt;
        let event_configuration = withdrawal_account_data.pda.event_configuration;
        let event_data = hash(&withdrawal_account_data.event.data.try_to_vec()?);

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &settings,
            round_number,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            withdrawal_account_info,
        )?;

        let withdrawal_status = withdrawal_account_data.meta.data.status;

        // Validate Token Setting Account
        if *token_settings_account_info.key != settings {
            return Err(ProgramError::InvalidArgument);
        }

        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        let name = &token_settings_account_data.name;
        let symbol = &token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let (mint, _) = token_settings_account_data
            .kind
            .into_solana()
            .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
            token_settings_account_info,
        )?;

        if token_settings_account_data.emergency {
            return Err(SolanaBridgeError::EmergencyEnabled.into());
        }

        let withdrawal_amount = get_withdrawal_amount(
            withdrawal_account_data.event.data.amount,
            token_settings_account_data.ever_decimals,
            token_settings_account_data.solana_decimals,
        );

        // Do we have enough signers.
        let sig_count = withdrawal_account_data
            .signers
            .iter()
            .filter(|vote| **vote == Vote::Confirm)
            .count() as u32;

        if sig_count == withdrawal_account_data.required_votes {
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
                            .checked_add(withdrawal_amount)
                            .ok_or(SolanaBridgeError::Overflow)?;

                    if withdrawal_amount > token_settings_account_data.withdrawal_limit
                        || token_settings_account_data.withdrawal_daily_amount
                            > token_settings_account_data.withdrawal_daily_limit
                    {
                        withdrawal_account_data.meta.data.status =
                            WithdrawalTokenStatus::WaitingForApprove;
                    } else {
                        make_multi_token_sol_transfer(
                            program_id,
                            vault_account_info,
                            token_program_info,
                            recipient_token_account_info,
                            &token_settings_account_data,
                            &mut withdrawal_account_data,
                            withdrawal_amount,
                        )?;
                    }

                    TokenSettings::pack(
                        token_settings_account_data,
                        &mut token_settings_account_info.data.borrow_mut(),
                    )?;

                    withdrawal_account_data.is_executed = true
                }
                WithdrawalTokenStatus::Pending => make_multi_token_sol_transfer(
                    program_id,
                    vault_account_info,
                    token_program_info,
                    recipient_token_account_info,
                    &token_settings_account_data,
                    &mut withdrawal_account_data,
                    withdrawal_amount,
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

        // Validate Setting Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        settings_account_data.guardian = new_guardian;

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

        // Validate Setting Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
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

        let mut settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        // Validate Setting Account
        let name = &settings_account_data.name;
        let symbol = &settings_account_data.symbol;
        let ever_decimals = settings_account_data.ever_decimals;
        let solana_decimals = settings_account_data.solana_decimals;
        let mint = match settings_account_data.kind {
            TokenKind::Ever { mint } => mint,
            TokenKind::Solana { mint, .. } => mint,
        };

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
            token_settings_account_info,
        )?;

        settings_account_data.deposit_limit = new_deposit_limit;

        TokenSettings::pack(
            settings_account_data,
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

        let mut settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        // Validate Setting Account
        let name = &settings_account_data.name;
        let symbol = &settings_account_data.symbol;
        let ever_decimals = settings_account_data.ever_decimals;
        let solana_decimals = settings_account_data.solana_decimals;
        let mint = match settings_account_data.kind {
            TokenKind::Ever { mint } => mint,
            TokenKind::Solana { mint, .. } => mint,
        };

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
            token_settings_account_info,
        )?;

        if let Some(new_withdrawal_limit) = new_withdrawal_limit {
            settings_account_data.withdrawal_limit = new_withdrawal_limit;
        }

        if let Some(new_withdrawal_daily_limit) = new_withdrawal_daily_limit {
            settings_account_data.withdrawal_daily_limit = new_withdrawal_daily_limit;
        }

        TokenSettings::pack(
            settings_account_data,
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

        // Validate Setting Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
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

        // Validate Setting Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
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

        // Validate Setting Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
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

        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        // Validate Setting Account
        let name = &token_settings_account_data.name;
        let symbol = &token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let mint = match token_settings_account_data.kind {
            TokenKind::Ever { mint } => mint,
            TokenKind::Solana { mint, .. } => mint,
        };

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
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

        let mut token_settings_account_data =
            TokenSettings::unpack(&token_settings_account_info.data.borrow())?;

        // Validate Setting Account
        let name = &token_settings_account_data.name;
        let symbol = &token_settings_account_data.symbol;
        let ever_decimals = token_settings_account_data.ever_decimals;
        let solana_decimals = token_settings_account_data.solana_decimals;
        let mint = match token_settings_account_data.kind {
            TokenKind::Ever { mint } => mint,
            TokenKind::Solana { mint, .. } => mint,
        };

        validate_token_settings_account(
            program_id,
            name,
            symbol,
            ever_decimals,
            solana_decimals,
            &mint,
            token_settings_account_info,
        )?;

        token_settings_account_data.emergency = false;

        TokenSettings::pack(
            token_settings_account_data,
            &mut token_settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }
}

fn make_multi_token_sol_transfer<'a>(
    program_id: &Pubkey,
    vault_account_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    recipient_token_account_info: &AccountInfo<'a>,
    settings_account_data: &TokenSettings,
    withdrawal_account_data: &mut WithdrawalMultiTokenSol,
    withdrawal_amount: u64,
) -> ProgramResult {
    // Validate Recipient Account
    let recipient_token_account_data =
        spl_token::state::Account::unpack(&recipient_token_account_info.data.borrow())?;

    if recipient_token_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    let recipient_token_address = withdrawal_account_data.event.data.recipient_address;

    if recipient_token_account_data.owner != recipient_token_address {
        return Err(ProgramError::InvalidArgument);
    }

    // Validate Vault Account
    let name = &settings_account_data.name;
    let symbol = &settings_account_data.name;
    let ever_decimals = settings_account_data.ever_decimals;
    let solana_decimals = settings_account_data.solana_decimals;
    let (mint, _) = settings_account_data
        .kind
        .into_solana()
        .map_err(|_| SolanaBridgeError::InvalidTokenKind)?;

    let vault_nonce = validate_vault_account(
        program_id,
        name,
        symbol,
        ever_decimals,
        solana_decimals,
        &mint,
        vault_account_info,
    )?;
    let vault_account_signer_seeds: &[&[_]] = &[br"vault", name.as_bytes(), &[vault_nonce]];

    if vault_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    let vault_account_data = spl_token::state::Account::unpack(&vault_account_info.data.borrow())?;

    // If vault balance is not enough
    if withdrawal_amount > vault_account_data.amount {
        withdrawal_account_data.meta.data.status = WithdrawalTokenStatus::Pending;
        return Ok(());
    }

    // Transfer tokens from Vault Account to Recipient Account
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
    program_id: &Pubkey,
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

    let recipient_token_address = withdrawal_account_data.event.data.recipient_address;

    if recipient_token_account_data.owner != recipient_token_address {
        return Err(ProgramError::InvalidArgument);
    }

    // Validate Mint Account
    let name = &settings_account_data.name;
    let symbol = &settings_account_data.name;
    let ever_decimals = settings_account_data.ever_decimals;
    let solana_decimals = settings_account_data.solana_decimals;

    let mint_nonce = validate_mint_account(
        program_id,
        name,
        symbol,
        ever_decimals,
        solana_decimals,
        mint_account_info,
    )?;
    let mint_account_signer_seeds: &[&[_]] = &[
        br"mint",
        name.as_bytes(),
        symbol.as_bytes(),
        &ever_decimals.to_le_bytes(),
        &solana_decimals.to_le_bytes(),
        &[mint_nonce],
    ];

    if mint_account_info.owner != &spl_token::id() {
        return Err(ProgramError::InvalidArgument);
    }

    // Mint EVER tokens to Recipient Account
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
