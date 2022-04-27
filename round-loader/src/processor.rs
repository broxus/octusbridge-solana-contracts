use borsh::BorshDeserialize;
use bridge_utils::state::{AccountKind, Proposal, PDA};
use bridge_utils::types::Vote;

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
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
        let instruction = RoundLoaderInstruction::try_from_slice(instruction_data).unwrap();

        match instruction {
            RoundLoaderInstruction::Initialize {
                round_number,
                round_end,
            } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(program_id, accounts, round_number, round_end)?;
            }
            RoundLoaderInstruction::CreateProposal {
                event_timestamp,
                event_transaction_lt,
            } => {
                msg!("Instruction: Create");
                Self::process_create_proposal(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                )?;
            }
            RoundLoaderInstruction::WriteProposal {
                event_timestamp,
                event_transaction_lt,
                offset,
                bytes,
            } => {
                msg!("Instruction: Write");
                Self::process_write_proposal(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    offset,
                    bytes,
                )?;
            }
            RoundLoaderInstruction::FinalizeProposal {
                event_timestamp,
                event_transaction_lt,
            } => {
                msg!("Instruction: Finalize");
                Self::process_finalize_proposal(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                )?;
            }
            RoundLoaderInstruction::VoteForProposal { vote } => {
                msg!("Instruction: Vote");
                Self::process_vote_for_proposal(program_id, accounts, vote)?;
            }
            RoundLoaderInstruction::ExecuteProposal => {
                msg!("Instruction: Execute");
                Self::process_execute_proposal(program_id, accounts)?;
            }
        };

        Ok(())
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        round_number: u32,
        round_end: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let initializer_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !initializer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            initializer_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        let settings_nonce = validate_settings_account(program_id, settings_account_info)?;
        let settings_account_signer_seeds: &[&[_]] = &[br"settings", &[settings_nonce]];

        // Create Settings Account
        invoke_signed(
            &system_instruction::create_account(
                initializer_account_info.key,
                settings_account_info.key,
                1.max(rent.minimum_balance(Settings::LEN)),
                Settings::LEN as u64,
                program_id,
            ),
            &[
                initializer_account_info.clone(),
                settings_account_info.clone(),
                system_program_info.clone(),
            ],
            &[settings_account_signer_seeds],
        )?;

        // Init Settings Account
        let settings_account_data = Settings {
            is_initialized: true,
            account_kind: AccountKind::Settings,
            round_number,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Validate Relay Round Account
        let relay_round_nonce =
            validate_relay_round_account(program_id, round_number, relay_round_account_info)?;
        let relay_round_account_signer_seeds: &[&[_]] =
            &[&round_number.to_le_bytes(), &[relay_round_nonce]];

        // Create Relay Round Account
        invoke_signed(
            &system_instruction::create_account(
                initializer_account_info.key,
                relay_round_account_info.key,
                1.max(rent.minimum_balance(RelayRound::LEN)),
                RelayRound::LEN as u64,
                program_id,
            ),
            &[
                initializer_account_info.clone(),
                relay_round_account_info.clone(),
                system_program_info.clone(),
            ],
            &[relay_round_account_signer_seeds],
        )?;

        // Init Relay Round Account
        let relay_round_account_data = RelayRound {
            is_initialized: true,
            account_kind: AccountKind::RelayRound,
            round_number,
            round_end,
            relays: vec![*initializer_account_info.key],
        };

        RelayRound::pack(
            relay_round_account_data,
            &mut relay_round_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_create_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let author = creator_account_info.key;
        let settings = get_associated_settings_address(program_id);

        // Validate Proposal Account
        let proposal_nonce = bridge_utils::helper::validate_proposal_account(
            program_id,
            author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            proposal_account_info,
        )?;
        let proposal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &author.to_bytes(),
            &settings.to_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &[proposal_nonce],
        ];

        // Create Proposal Account
        invoke_signed(
            &system_instruction::create_account(
                creator_account_info.key,
                proposal_account_info.key,
                1.max(rent.minimum_balance(RelayRoundProposal::LEN)),
                RelayRoundProposal::LEN as u64,
                program_id,
            ),
            &[
                creator_account_info.clone(),
                proposal_account_info.clone(),
                system_program_info.clone(),
            ],
            &[proposal_account_signer_seeds],
        )?;

        Ok(())
    }

    fn process_write_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
        offset: u32,
        bytes: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let settings = get_associated_settings_address(program_id);

        // Validate Proposal Account
        bridge_utils::helper::validate_proposal_account(
            program_id,
            creator_account_info.key,
            &settings,
            event_timestamp,
            event_transaction_lt,
            proposal_account_info,
        )?;

        if proposal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let proposal = RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;
        if proposal.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        write_proposal_data(
            &mut proposal_account_info.data.borrow_mut(),
            offset as usize,
            &bytes,
        )?;

        Ok(())
    }

    fn process_finalize_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        validate_settings_account(program_id, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let round_number = settings_account_data.round_number;

        // Validate Relay Round Account
        validate_relay_round_account(program_id, round_number, relay_round_account_info)?;

        if relay_round_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;

        if relay_round_account_data.round_end <= clock.unix_timestamp as u32 {
            return Err(RoundLoaderError::RelayRoundExpired.into());
        }

        let required_votes = (relay_round_account_data.relays.len() * 2 / 3 + 1) as u32;

        // Validate Proposal Account
        bridge_utils::helper::validate_proposal_account(
            program_id,
            creator_account_info.key,
            settings_account_info.key,
            event_timestamp,
            event_transaction_lt,
            proposal_account_info,
        )?;

        if proposal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut proposal =
            RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;

        if proposal.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let pda = PDA {
            author: *creator_account_info.key,
            settings: *settings_account_info.key,
            event_timestamp,
            event_transaction_lt,
        };

        proposal.is_initialized = true;
        proposal.account_kind = AccountKind::Proposal;
        proposal.round_number = round_number;
        proposal.required_votes = required_votes;
        proposal.pda = pda;
        proposal.signers = vec![Vote::None; proposal.event.data.relays.len()];

        proposal.meta = RelayRoundProposalMetaWithLen::new();

        RelayRoundProposal::pack(proposal, &mut proposal_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_vote_for_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        vote: Vote,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let voter_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;

        if !voter_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Proposal Account
        if proposal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut proposal_account_data = Proposal::unpack(&proposal_account_info.data.borrow())?;
        let author = proposal_account_data.pda.author;
        let settings = proposal_account_data.pda.settings;
        let event_timestamp = proposal_account_data.pda.event_timestamp;
        let event_transaction_lt = proposal_account_data.pda.event_transaction_lt;

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &author,
            &settings,
            event_timestamp,
            event_transaction_lt,
            proposal_account_info,
        )?;

        let round_number = proposal_account_data.round_number;

        // Validate Relay Round Account
        validate_relay_round_account(program_id, round_number, relay_round_account_info)?;

        if relay_round_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;

        // Vote for proposal request
        let index = relay_round_account_data
            .relays
            .iter()
            .position(|pubkey| pubkey == voter_account_info.key)
            .ok_or(RoundLoaderError::InvalidRelay)?;
        proposal_account_data.signers[index] = vote;

        Proposal::pack(
            proposal_account_data,
            &mut proposal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_execute_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        // Validate Settings Account
        validate_settings_account(program_id, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        let mut proposal = RelayRoundProposal::unpack(&proposal_account_info.data.borrow())?;

        // Do we have enough signers.
        let sig_count = proposal
            .signers
            .iter()
            .filter(|vote| **vote == Vote::Confirm)
            .count() as u32;

        if !proposal.meta.data.is_executed && sig_count >= proposal.required_votes {
            // Validate a new Relay Round Account
            let round_number = proposal.event.data.round_num;
            let nonce =
                validate_relay_round_account(program_id, round_number, relay_round_account_info)?;

            let relay_round_account_signer_seeds: &[&[_]] =
                &[&round_number.to_le_bytes(), &[nonce]];

            // Create a new Relay Round Account
            invoke_signed(
                &system_instruction::create_account(
                    funder_account_info.key,
                    relay_round_account_info.key,
                    1.max(rent.minimum_balance(RelayRound::LEN)),
                    RelayRound::LEN as u64,
                    program_id,
                ),
                &[
                    funder_account_info.clone(),
                    relay_round_account_info.clone(),
                    system_program_info.clone(),
                ],
                &[relay_round_account_signer_seeds],
            )?;

            // Init a new Relay Round Account
            let relay_round_account_data = RelayRound {
                is_initialized: true,
                account_kind: AccountKind::RelayRound,
                round_number,
                round_end: proposal.event.data.round_end,
                relays: proposal.event.data.relays.clone(),
            };

            RelayRound::pack(
                relay_round_account_data,
                &mut relay_round_account_info.data.borrow_mut(),
            )?;

            // Update Settings Account
            settings_account_data.round_number = round_number;

            Settings::pack(
                settings_account_data,
                &mut settings_account_info.data.borrow_mut(),
            )?;

            proposal.meta.data.is_executed = true;
        }

        // Update Proposal Account
        RelayRoundProposal::pack(proposal, &mut proposal_account_info.data.borrow_mut())?;

        Ok(())
    }
}

fn write_proposal_data(data: &mut [u8], offset: usize, bytes: &[u8]) -> Result<(), ProgramError> {
    let offset = LOAD_DATA_BEGIN_OFFSET + offset;

    let len = bytes.len();
    if LOAD_DATA_END_OFFSET < offset + len {
        msg!(
            "Write overflow: {} < {}",
            LOAD_DATA_END_OFFSET,
            offset + len
        );
        return Err(ProgramError::AccountDataTooSmall);
    }

    data[offset..offset + len].copy_from_slice(bytes);

    Ok(())
}
