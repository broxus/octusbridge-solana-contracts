use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::state::{AccountKind, Proposal, PDA};
use bridge_utils::types::{Vote, RELAY_REPARATION};

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::hash::{hash, Hash};
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
        let instruction = RoundLoaderInstruction::try_from_slice(instruction_data)?;

        match instruction {
            RoundLoaderInstruction::Initialize {
                genesis_round_number,
                round_submitter,
                min_required_votes,
                round_ttl,
            } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(
                    program_id,
                    accounts,
                    genesis_round_number,
                    round_submitter,
                    min_required_votes,
                    round_ttl,
                )?;
            }
            RoundLoaderInstruction::UpdateSettings {
                current_round_number,
                round_submitter,
                min_required_votes,
                round_ttl,
            } => {
                msg!("Instruction: Update Settings");
                Self::process_update_settings(
                    program_id,
                    accounts,
                    current_round_number,
                    round_submitter,
                    min_required_votes,
                    round_ttl,
                )?;
            }
            RoundLoaderInstruction::CreateRelayRound {
                round_number,
                relays,
                round_end,
            } => {
                msg!("Instruction: Create Relay Round");
                Self::process_create_relay_round(
                    program_id,
                    accounts,
                    round_number,
                    relays,
                    round_end,
                )?;
            }
            RoundLoaderInstruction::CreateProposal {
                event_timestamp,
                event_transaction_lt,
                event_configuration,
                event_data,
            } => {
                msg!("Instruction: Create");
                Self::process_create_proposal(
                    program_id,
                    accounts,
                    event_timestamp,
                    event_transaction_lt,
                    event_configuration,
                    event_data,
                )?;
            }
            RoundLoaderInstruction::WriteProposal { offset, bytes } => {
                msg!("Instruction: Write");
                Self::process_write_proposal(program_id, accounts, offset, bytes)?;
            }
            RoundLoaderInstruction::FinalizeProposal => {
                msg!("Instruction: Finalize");
                Self::process_finalize_proposal(program_id, accounts)?;
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
        genesis_round_number: u32,
        round_submitter: Pubkey,
        min_required_votes: u32,
        round_ttl: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let initializer_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
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
            current_round_number: genesis_round_number,
            round_submitter,
            min_required_votes,
            round_ttl,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_update_settings(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        current_round_number: Option<u32>,
        round_submitter: Option<Pubkey>,
        min_required_votes: Option<u32>,
        round_ttl: Option<u32>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let author_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;

        if !author_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::helper::validate_programdata_account(
            program_id,
            programdata_account_info.key,
        )?;
        bridge_utils::helper::validate_initializer_account(
            author_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;

        if let Some(current_round_number) = current_round_number {
            settings_account_data.current_round_number = current_round_number;
        }

        if let Some(round_submitter) = round_submitter {
            settings_account_data.round_submitter = round_submitter;
        }

        if let Some(min_required_votes) = min_required_votes {
            settings_account_data.min_required_votes = min_required_votes;
        }

        if let Some(round_ttl) = round_ttl {
            settings_account_data.round_ttl = round_ttl;
        }

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_create_relay_round(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        round_number: u32,
        relays: Vec<Pubkey>,
        round_end: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        if settings_account_data.round_submitter != *creator_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        if settings_account_data.current_round_number != 0
            && settings_account_data.current_round_number > round_number
        {
            return Err(RoundLoaderError::InvalidRelayRound.into());
        }

        settings_account_data.current_round_number = round_number;

        // Validate Relay Round Account
        let relay_round_nonce =
            validate_relay_round_account(program_id, round_number, relay_round_account_info)?;
        let relay_round_account_signer_seeds: &[&[_]] =
            &[&round_number.to_le_bytes(), &[relay_round_nonce]];

        // Create Relay Round Account
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

        let round_end = round_end + settings_account_data.round_ttl;

        // Init Relay Round Account
        let relay_round_account_data = RelayRound {
            is_initialized: true,
            account_kind: AccountKind::RelayRound,
            round_number,
            round_end,
            relays,
        };

        RelayRound::pack(
            relay_round_account_data,
            &mut relay_round_account_info.data.borrow_mut(),
        )?;

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_create_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        event_timestamp: u32,
        event_transaction_lt: u64,
        event_configuration: Pubkey,
        event_data: Hash,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let settings = bridge_utils::helper::get_associated_settings_address(program_id);

        // Validate Proposal Account
        let proposal_nonce = bridge_utils::helper::validate_proposal_account(
            program_id,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
            proposal_account_info,
        )?;
        let proposal_account_signer_seeds: &[&[_]] = &[
            br"proposal",
            &settings.to_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data.to_bytes(),
            &[proposal_nonce],
        ];

        // Create Proposal Account
        invoke_signed(
            &system_instruction::create_account(
                funder_account_info.key,
                proposal_account_info.key,
                1.max(rent.minimum_balance(RelayRoundProposal::LEN)),
                RelayRoundProposal::LEN as u64,
                program_id,
            ),
            &[
                funder_account_info.clone(),
                proposal_account_info.clone(),
                system_program_info.clone(),
            ],
            &[proposal_account_signer_seeds],
        )?;

        // Init Proposal Account
        let proposal_account_data = RelayRoundProposal {
            account_kind: AccountKind::Proposal,
            author: *creator_account_info.key,
            pda: PDA {
                settings,
                event_timestamp,
                event_transaction_lt,
                event_configuration,
            },
            ..Default::default()
        };

        RelayRoundProposal::pack(
            proposal_account_data,
            &mut proposal_account_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    fn process_write_proposal(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        offset: u32,
        bytes: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
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

    fn process_finalize_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

        if settings_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let round_number = settings_account_data.current_round_number;

        // Validate Relay Round Account
        validate_relay_round_account(program_id, round_number, relay_round_account_info)?;

        if relay_round_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let relay_round_account_data = RelayRound::unpack(&relay_round_account_info.data.borrow())?;

        let mut required_votes = (relay_round_account_data.relays.len() * 2 / 3 + 1) as u32;
        if settings_account_data.min_required_votes > required_votes {
            required_votes = settings_account_data.min_required_votes;
        }

        let mut proposal =
            RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;

        if proposal.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        if proposal.author != *creator_account_info.key {
            return Err(ProgramError::IllegalOwner);
        }

        if round_number >= proposal.event.data.round_num {
            return Err(RoundLoaderError::InvalidProposalRoundNumber.into());
        }

        proposal.is_initialized = true;
        proposal.round_number = round_number;
        proposal.required_votes = required_votes;
        proposal.signers = vec![Vote::None; relay_round_account_data.relays.len()];

        proposal.meta = RelayRoundProposalMetaWithLen::default();

        RelayRoundProposal::pack(proposal, &mut proposal_account_info.data.borrow_mut())?;

        // Send voting reparation for Relay to withdrawal account
        invoke(
            &system_instruction::transfer(
                creator_account_info.key,
                proposal_account_info.key,
                RELAY_REPARATION * relay_round_account_data.relays.len() as u64,
            ),
            &[
                creator_account_info.clone(),
                proposal_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;

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

        // Validate vote
        if vote == Vote::None {
            return Err(RoundLoaderError::InvalidVote.into());
        }

        // Validate Proposal Account
        if proposal_account_info.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }

        let mut proposal_account_data =
            Proposal::unpack_from_slice(&proposal_account_info.data.borrow())?;

        let settings = proposal_account_data.pda.settings;
        let event_timestamp = proposal_account_data.pda.event_timestamp;
        let event_transaction_lt = proposal_account_data.pda.event_transaction_lt;
        let event_configuration = proposal_account_data.pda.event_configuration;

        let event_data = hash(&proposal_account_data.event.try_to_vec()?[4..]);

        bridge_utils::helper::validate_proposal_account(
            program_id,
            &settings,
            event_timestamp,
            event_transaction_lt,
            &event_configuration,
            &event_data,
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

        if proposal_account_data.signers[index] == Vote::None {
            // Vote for proposal
            proposal_account_data.signers[index] = vote;
            proposal_account_data.pack_into_slice(&mut proposal_account_info.data.borrow_mut());

            // Get back voting reparation to Relay
            **proposal_account_info.try_borrow_mut_lamports()? -= RELAY_REPARATION;
            **voter_account_info.try_borrow_mut_lamports()? += RELAY_REPARATION;
        }

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
        bridge_utils::helper::validate_settings_account(program_id, settings_account_info)?;

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

        if !proposal.is_executed && sig_count >= proposal.required_votes {
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

            let round_end = proposal.event.data.round_end + settings_account_data.round_ttl;

            // Init a new Relay Round Account
            let relay_round_account_data = RelayRound {
                is_initialized: true,
                account_kind: AccountKind::RelayRound,
                round_number,
                round_end,
                relays: proposal
                    .event
                    .data
                    .relays
                    .iter()
                    .cloned()
                    .map(|relay| Pubkey::new_from_array(relay.try_into().unwrap()))
                    .collect(),
            };

            RelayRound::pack(
                relay_round_account_data,
                &mut relay_round_account_info.data.borrow_mut(),
            )?;

            // Update Settings Account
            settings_account_data.current_round_number = round_number;

            Settings::pack(
                settings_account_data,
                &mut settings_account_info.data.borrow_mut(),
            )?;

            proposal.is_executed = true;
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
