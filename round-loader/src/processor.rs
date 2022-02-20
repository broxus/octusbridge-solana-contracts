use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::{
    RelayRound, RelayRoundProposal, RoundLoaderError, RoundLoaderInstruction, Settings,
    LOAD_DATA_BEGIN_OFFSET, LOAD_DATA_END_OFFSET,
};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = RoundLoaderInstruction::try_from_slice(instruction_data).unwrap();

        match instruction {
            RoundLoaderInstruction::Initialize { round, round_ttl } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(program_id, accounts, round, round_ttl)?;
            }
            RoundLoaderInstruction::CreateProposal { round } => {
                msg!("Instruction: Create");
                Self::process_create_proposal(program_id, accounts, round)?;
            }
            RoundLoaderInstruction::WriteProposal {
                round,
                offset,
                bytes,
            } => {
                msg!("Instruction: Write");
                Self::process_write_proposal(program_id, accounts, round, offset, bytes)?;
            }
            RoundLoaderInstruction::FinalizeProposal { round } => {
                msg!("Instruction: Finalize");
                Self::process_finalize_proposal(program_id, accounts, round)?;
            }
            RoundLoaderInstruction::Vote => {
                msg!("Instruction: Vote");
                Self::process_vote_for_proposal(program_id, accounts)?;
            }
        };

        Ok(())
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        round: u32,
        round_ttl: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_programdata_account(program_id, programdata_account_info.key)?;

        bridge_utils::validate_creator_account(creator_account_info.key, programdata_account_info)?;

        // Create Settings Account
        let settings_nonce =
            bridge_utils::validate_settings_account(program_id, settings_account_info.key)?;
        let settings_account_signer_seeds: &[&[_]] = &[b"settings", &[settings_nonce]];

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
            round_number: round,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Create the first Relay Round Account
        let round_nonce = bridge_utils::validate_round_relay_account(
            program_id,
            relay_round_account_info.key,
            round,
        )?;
        let relay_round_account_signer_seeds: &[&[_]] = &[&round.to_le_bytes(), &[round_nonce]];

        bridge_utils::fund_account(
            relay_round_account_info,
            funder_account_info,
            system_program_info,
            RelayRound::LEN,
        )?;

        bridge_utils::create_account(
            program_id,
            relay_round_account_info,
            system_program_info,
            relay_round_account_signer_seeds,
            RelayRound::LEN,
        )?;

        let relay_round_account_data = RelayRound {
            is_initialized: true,
            round_number: round,
            round_ttl,
            relays: vec![*creator_account_info.key],
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
        round: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let current_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_settings_account(program_id, settings_account_info.key)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        bridge_utils::validate_round_relay_account(
            program_id,
            current_round_account_info.key,
            current_round,
        )?;

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;

        if !current_round_account_data
            .relays
            .contains(creator_account_info.key)
        {
            return Err(RoundLoaderError::InvalidRelay.into());
        }

        // Create Proposal Account
        let nonce = bridge_utils::validate_proposal_account(
            program_id,
            creator_account_info.key,
            proposal_account_info.key,
            round,
        )?;
        let proposal_account_signer_seeds: &[&[_]] = &[
            &creator_account_info.key.to_bytes(),
            &round.to_le_bytes(),
            &[nonce],
        ];

        bridge_utils::fund_account(
            proposal_account_info,
            funder_account_info,
            system_program_info,
            RelayRoundProposal::LEN,
        )?;

        bridge_utils::create_account(
            program_id,
            proposal_account_info,
            system_program_info,
            proposal_account_signer_seeds,
            RelayRoundProposal::LEN,
        )?;

        Ok(())
    }

    fn process_write_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        round: u32,
        offset: u32,
        bytes: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_proposal_account(
            program_id,
            creator_account_info.key,
            proposal_account_info.key,
            round,
        )?;

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
        round: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let creator_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let current_round_account_info = next_account_info(account_info_iter)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_settings_account(program_id, settings_account_info.key)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        bridge_utils::validate_round_relay_account(
            program_id,
            current_round_account_info.key,
            current_round,
        )?;

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;
        let required_votes = (current_round_account_data.relays.len() * 2 / 3 + 1) as u32;

        bridge_utils::validate_proposal_account(
            program_id,
            creator_account_info.key,
            proposal_account_info.key,
            round,
        )?;

        let mut proposal =
            RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;

        if proposal.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        proposal.is_initialized = true;
        proposal.author = *creator_account_info.key;
        proposal.round_number = round;
        proposal.required_votes = required_votes;

        RelayRoundProposal::pack(proposal, &mut proposal_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_vote_for_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let voter_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let new_round_account_info = next_account_info(account_info_iter)?;
        let current_round_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !voter_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_settings_account(program_id, settings_account_info.key)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        bridge_utils::validate_round_relay_account(
            program_id,
            current_round_account_info.key,
            current_round,
        )?;

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;

        if !current_round_account_data
            .relays
            .contains(voter_account_info.key)
        {
            return Err(RoundLoaderError::InvalidRelay.into());
        }

        if proposal_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }

        let mut proposal = RelayRoundProposal::unpack(&proposal_account_info.data.borrow())?;

        if proposal.voters.contains(voter_account_info.key) {
            return Err(RoundLoaderError::RelayAlreadyVoted.into());
        }

        proposal.voters.push(*voter_account_info.key);

        if !proposal.is_executed && proposal.voters.len() as u32 >= proposal.required_votes {
            // Create a new Relay Round Account
            let round_nonce = bridge_utils::validate_round_relay_account(
                program_id,
                new_round_account_info.key,
                proposal.round_number,
            )?;
            let relay_round_account_signer_seeds: &[&[_]] =
                &[&proposal.round_number.to_le_bytes(), &[round_nonce]];

            bridge_utils::fund_account(
                new_round_account_info,
                funder_account_info,
                system_program_info,
                RelayRound::LEN,
            )?;

            bridge_utils::create_account(
                program_id,
                new_round_account_info,
                system_program_info,
                relay_round_account_signer_seeds,
                RelayRound::LEN,
            )?;

            let relay_round_account_data = RelayRound {
                is_initialized: true,
                round_number: proposal.round_number,
                round_ttl: proposal.round_ttl,
                relays: proposal.relays.clone(),
            };

            RelayRound::pack(
                relay_round_account_data,
                &mut new_round_account_info.data.borrow_mut(),
            )?;

            settings_account_data.round_number = proposal.round_number;

            Settings::pack(
                settings_account_data,
                &mut settings_account_info.data.borrow_mut(),
            )?;

            proposal.is_executed = true;
        }

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
