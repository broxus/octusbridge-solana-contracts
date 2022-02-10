use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::{
    RelayRoundProposal, RoundLoaderError, RoundLoaderInstruction, LOAD_DATA_BEGIN_OFFSET,
    LOAD_DATA_END_OFFSET,
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

    fn process_create_proposal(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        round: u32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (pda, nonce) = Pubkey::find_program_address(
            &[&relay_account_info.key.to_bytes(), &round.to_le_bytes()],
            program_id,
        );

        if pda != *proposal_account_info.key {
            msg!("Error: Associated address does not match seed derivation");
            return Err(ProgramError::InvalidSeeds);
        }

        let rent = &Rent::from_account_info(rent_sysvar_info)?;
        let required_lamports = rent
            .minimum_balance(RelayRoundProposal::LEN)
            .max(1)
            .saturating_sub(proposal_account_info.lamports());

        if required_lamports > 0 {
            msg!(
                "Transfer {} lamports to the associated proposal account",
                required_lamports
            );
            invoke(
                &system_instruction::transfer(
                    relay_account_info.key,
                    proposal_account_info.key,
                    required_lamports,
                ),
                &[
                    relay_account_info.clone(),
                    proposal_account_info.clone(),
                    system_program_info.clone(),
                ],
            )?;
        }

        let proposal_account_signer_seeds: &[&[_]] = &[
            &relay_account_info.key.to_bytes(),
            &round.to_le_bytes(),
            &[nonce],
        ];

        msg!("Allocate space for the proposal account");
        invoke_signed(
            &system_instruction::allocate(
                proposal_account_info.key,
                RelayRoundProposal::LEN as u64,
            ),
            &[proposal_account_info.clone(), system_program_info.clone()],
            &[&proposal_account_signer_seeds[..]],
        )?;

        msg!("Assign the proposal account to the round-loader program");
        invoke_signed(
            &system_instruction::assign(proposal_account_info.key, program_id),
            &[proposal_account_info.clone(), system_program_info.clone()],
            &[&proposal_account_signer_seeds[..]],
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

        let (pda, _) = Pubkey::find_program_address(
            &[&relay_account_info.key.to_bytes(), &round.to_le_bytes()],
            program_id,
        );

        if pda != *proposal_account_info.key {
            msg!("Error: Associated address does not match seed derivation");
            return Err(ProgramError::InvalidSeeds);
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

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (pda, _) = Pubkey::find_program_address(
            &[&relay_account_info.key.to_bytes(), &round.to_le_bytes()],
            program_id,
        );

        if pda != *proposal_account_info.key {
            msg!("Error: Associated address does not match seed derivation");
            return Err(ProgramError::InvalidSeeds);
        }

        let mut proposal =
            RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;

        if proposal.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        proposal.is_initialized = true;
        proposal.author = *relay_account_info.key;

        // TODO: round_number
        // TODO: required_votes

        RelayRoundProposal::pack(proposal, &mut proposal_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_vote_for_proposal(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let relay_account_info = next_account_info(account_info_iter)?;
        let proposal_account_info = next_account_info(account_info_iter)?;

        if !relay_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut proposal =
            RelayRoundProposal::unpack_unchecked(&proposal_account_info.data.borrow())?;

        if !proposal.is_initialized {
            return Err(ProgramError::UninitializedAccount);
        }

        if !proposal.relays.contains(relay_account_info.key) {
            return Err(RoundLoaderError::InvalidRelay.into());
        }

        if proposal.voters.contains(relay_account_info.key) {
            return Err(RoundLoaderError::RelayAlreadyVoted.into());
        }

        proposal.voters.push(relay_account_info.key.clone());

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
        return Err(RoundLoaderError::ProposalRelaysDataTooSmall.into());
    }

    data[offset..offset + len].copy_from_slice(bytes);

    Ok(())
}
