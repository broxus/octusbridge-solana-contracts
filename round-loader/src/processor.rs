use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

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
        round_ttl: i64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let funder_account_info = next_account_info(account_info_iter)?;
        let creator_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
        let programdata_account_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        bridge_utils::validate_programdata_account(program_id, programdata_account_info.key)?;
        bridge_utils::validate_initializer_account(
            creator_account_info.key,
            programdata_account_info,
        )?;

        // Validate Settings Account
        let (settings_account, settings_nonce) =
            Pubkey::find_program_address(&[br"settings"], program_id);
        if settings_account != *settings_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

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
            round_number: round,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Validate Relay Round Account
        let (relay_round_account, relay_round_nonce) =
            Pubkey::find_program_address(&[&round.to_le_bytes()], program_id);
        if relay_round_account != *relay_round_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let relay_round_account_signer_seeds: &[&[_]] =
            &[&round.to_le_bytes(), &[relay_round_nonce]];

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

        // Init Relay Round Account
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
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !creator_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let (settings_account, _nonce) = Pubkey::find_program_address(&[br"settings"], program_id);
        if settings_account != *settings_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        // Validate current Relay Round Account
        let (current_relay_round_account, _nonce) =
            Pubkey::find_program_address(&[&current_round.to_le_bytes()], program_id);
        if current_relay_round_account != *current_round_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;

        if !current_round_account_data
            .relays
            .contains(creator_account_info.key)
        {
            return Err(RoundLoaderError::InvalidRelay.into());
        }

        // Validate Proposal Account
        let (proposal_account, proposal_nonce) = Pubkey::find_program_address(
            &[&creator_account_info.key.to_bytes(), &round.to_le_bytes()],
            program_id,
        );
        if proposal_account != *proposal_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let proposal_account_signer_seeds: &[&[_]] = &[
            &creator_account_info.key.to_bytes(),
            &round.to_le_bytes(),
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

        // Validate Proposal Account
        let (proposal_account, _nonce) = Pubkey::find_program_address(
            &[&creator_account_info.key.to_bytes(), &round.to_le_bytes()],
            program_id,
        );
        if proposal_account != *proposal_account_info.key {
            return Err(ProgramError::InvalidAccountData);
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

        // Validate Settings Account
        let (settings_account, _nonce) = Pubkey::find_program_address(&[br"settings"], program_id);
        if settings_account != *settings_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        // Validate current Relay Round Account
        let (current_relay_round_account, _nonce) =
            Pubkey::find_program_address(&[&current_round.to_le_bytes()], program_id);
        if current_relay_round_account != *current_round_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;
        let required_votes = (current_round_account_data.relays.len() * 2 / 3 + 1) as u32;

        // Validate Proposal Account
        let (proposal_account, _nonce) = Pubkey::find_program_address(
            &[&creator_account_info.key.to_bytes(), &round.to_le_bytes()],
            program_id,
        );
        if proposal_account != *proposal_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

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
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        if !voter_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Validate Settings Account
        let (settings_account, _nonce) = Pubkey::find_program_address(&[br"settings"], program_id);
        if settings_account != *settings_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        // Validate current Relay Round Account
        let (current_relay_round_account, _nonce) =
            Pubkey::find_program_address(&[&current_round.to_le_bytes()], program_id);
        if current_relay_round_account != *current_round_account_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

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
            // Validate a new Relay Round Account
            let (new_relay_round_account, new_relay_round_nonce) =
                Pubkey::find_program_address(&[&proposal.round_number.to_le_bytes()], program_id);
            if new_relay_round_account != *new_round_account_info.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let new_relay_round_account_signer_seeds: &[&[_]] = &[
                &proposal.round_number.to_le_bytes(),
                &[new_relay_round_nonce],
            ];

            // Create a new Relay Round Account
            invoke_signed(
                &system_instruction::create_account(
                    funder_account_info.key,
                    new_round_account_info.key,
                    1.max(rent.minimum_balance(RelayRound::LEN)),
                    RelayRound::LEN as u64,
                    program_id,
                ),
                &[
                    funder_account_info.clone(),
                    new_round_account_info.clone(),
                    system_program_info.clone(),
                ],
                &[new_relay_round_account_signer_seeds],
            )?;

            // Init a new Relay Round Account
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

            // Update Settings Account
            settings_account_data.round_number = proposal.round_number;

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
