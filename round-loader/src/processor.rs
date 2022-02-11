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

        let authority_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let relay_round_account_info = next_account_info(account_info_iter)?;
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
        let settings_nonce = validate_settings_account(program_id, settings_account_info.key)?;
        let settings_account_signer_seeds: &[&[_]] = &[b"settings", &[settings_nonce]];

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
            round_number: round,
        };

        Settings::pack(
            settings_account_data,
            &mut settings_account_info.data.borrow_mut(),
        )?;

        // Create the first Relay Round account
        let round_nonce =
            validate_round_relay_account(program_id, relay_round_account_info.key, round)?;
        let relay_round_account_signer_seeds: &[&[_]] = &[&round.to_le_bytes(), &[round_nonce]];

        create_account(
            program_id,
            authority_account_info,
            relay_round_account_info,
            system_program_info,
            RelayRound::LEN,
            relay_round_account_signer_seeds,
            rent,
        )?;

        let relay_round_account_data = RelayRound {
            is_initialized: true,
            round_number: round,
            round_ttl,
            relays: vec![*authority_account_info.key],
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

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let current_round_account_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        validate_settings_account(program_id, settings_account_info.key)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        validate_round_relay_account(program_id, current_round_account_info.key, current_round)?;

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;

        if !current_round_account_data
            .relays
            .contains(relay_account_info.key)
        {
            return Err(RoundLoaderError::InvalidRelay.into());
        }

        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        // Create proposal account
        let nonce = validate_proposal_account(
            program_id,
            relay_account_info.key,
            proposal_account_info.key,
            round,
        )?;
        let proposal_account_signer_seeds: &[&[_]] = &[
            &relay_account_info.key.to_bytes(),
            &round.to_le_bytes(),
            &[nonce],
        ];

        create_account(
            program_id,
            relay_account_info,
            proposal_account_info,
            system_program_info,
            RelayRoundProposal::LEN,
            proposal_account_signer_seeds,
            rent,
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

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        validate_proposal_account(
            program_id,
            relay_account_info.key,
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

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let current_round_account_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        validate_settings_account(program_id, settings_account_info.key)?;

        let settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        validate_round_relay_account(program_id, current_round_account_info.key, current_round)?;

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;
        let required_votes = (current_round_account_data.relays.len() * 2 / 3 + 1) as u32;

        validate_proposal_account(
            program_id,
            relay_account_info.key,
            proposal_account_info.key,
            round,
        )?;

        let mut proposal =
            RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;

        if proposal.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        proposal.is_initialized = true;
        proposal.author = *relay_account_info.key;
        proposal.round_number = round;
        proposal.required_votes = required_votes;

        RelayRoundProposal::pack(proposal, &mut proposal_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_vote_for_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let settings_account_info = next_account_info(account_info_iter)?;
        let new_round_account_info = next_account_info(account_info_iter)?;
        let current_round_account_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        validate_settings_account(program_id, settings_account_info.key)?;

        let mut settings_account_data = Settings::unpack(&settings_account_info.data.borrow())?;
        let current_round = settings_account_data.round_number;

        validate_round_relay_account(program_id, current_round_account_info.key, current_round)?;

        let current_round_account_data =
            RelayRound::unpack(&current_round_account_info.data.borrow())?;

        if !current_round_account_data
            .relays
            .contains(relay_account_info.key)
        {
            return Err(RoundLoaderError::InvalidRelay.into());
        }

        if proposal_account_info.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }

        let mut proposal = RelayRoundProposal::unpack(&proposal_account_info.data.borrow())?;

        if proposal.voters.contains(relay_account_info.key) {
            return Err(RoundLoaderError::RelayAlreadyVoted.into());
        }

        proposal.voters.push(*relay_account_info.key);

        if !proposal.is_executed && proposal.voters.len() as u32 >= proposal.required_votes {
            let rent = &Rent::from_account_info(rent_sysvar_info)?;

            // Create a new Relay Round account
            let round_nonce = validate_round_relay_account(
                program_id,
                new_round_account_info.key,
                proposal.round_number,
            )?;
            let relay_round_account_signer_seeds: &[&[_]] =
                &[&proposal.round_number.to_le_bytes(), &[round_nonce]];

            create_account(
                program_id,
                relay_account_info,
                new_round_account_info,
                system_program_info,
                RelayRound::LEN,
                relay_round_account_signer_seeds,
                rent,
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
    settings_account: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(&[b"settings"], program_id);

    if pda != *settings_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

fn validate_round_relay_account(
    program_id: &Pubkey,
    round_relay_account: &Pubkey,
    round: u32,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(&[&round.to_le_bytes()], program_id);

    if pda != *round_relay_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

fn validate_proposal_account(
    program_id: &Pubkey,
    relay_account: &Pubkey,
    proposal_account: &Pubkey,
    round: u32,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(
        &[&relay_account.to_bytes(), &round.to_le_bytes()],
        program_id,
    );
    if pda != *proposal_account {
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
