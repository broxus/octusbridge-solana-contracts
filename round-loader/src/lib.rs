use borsh::BorshSerialize;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::sysvar::rent;

mod error;
mod instruction;
mod processor;
mod state;
mod utils;

pub use self::error::*;
pub use self::instruction::*;
pub use self::processor::*;
pub use self::state::*;
pub use self::utils::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("RoundLoaderPubKey");

pub fn get_associated_proposal_address_and_bump_seed(
    relay_address: &Pubkey,
    program_id: &Pubkey,
    round: u32,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&relay_address.to_bytes(), &round.to_le_bytes()],
        program_id,
    )
}

pub fn get_associated_proposal_address(relay_address: &Pubkey, round: u32) -> Pubkey {
    get_associated_proposal_address_and_bump_seed(relay_address, &id(), round).0
}

pub fn create_proposal(relay_pubkey: &Pubkey, round: u32) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(relay_pubkey, round);

    let data = RoundLoaderInstruction::CreateProposal { round }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

pub fn write_proposal(
    relay_pubkey: &Pubkey,
    round: u32,
    offset: u32,
    bytes: Vec<u8>,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(relay_pubkey, round);

    let data = RoundLoaderInstruction::WriteProposal {
        round,
        offset,
        bytes,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
        ],
        data,
    }
}

pub fn finalize_proposal(relay_pubkey: &Pubkey, round: u32) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(relay_pubkey, round);

    let data = RoundLoaderInstruction::FinalizeProposal { round }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
        ],
        data,
    }
}

pub fn vote_for_proposal(relay_pubkey: &Pubkey, author_pubkey: &Pubkey, round: u32) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(author_pubkey, round);

    let data = RoundLoaderInstruction::Vote.try_to_vec().expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
        ],
        data,
    }
}
