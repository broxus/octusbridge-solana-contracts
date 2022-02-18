use borsh::BorshSerialize;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{bpf_loader_upgradeable, system_program};

mod error;
mod instruction;
mod processor;
mod state;

pub use self::error::*;
pub use self::instruction::*;
pub use self::processor::*;
pub use self::state::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("4mTYQNUNZMrc9wvcQhyYdFKX4wBjmgytLtHWeyzZue5i");

pub fn get_associated_settings_address() -> Pubkey {
    Pubkey::find_program_address(&[b"settings"], &id()).0
}

pub fn get_program_data_address() -> Pubkey {
    Pubkey::find_program_address(&[id().as_ref()], &bpf_loader_upgradeable::id()).0
}

pub fn get_associated_relay_round_address(round: u32) -> Pubkey {
    Pubkey::find_program_address(&[&round.to_le_bytes()], &id()).0
}

pub fn get_associated_proposal_address(address: &Pubkey, round: u32) -> Pubkey {
    Pubkey::find_program_address(&[&address.to_bytes(), &round.to_le_bytes()], &id()).0
}

pub fn initialize(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    round: u32,
    round_ttl: u32,
) -> Instruction {
    let setting_pubkey = get_associated_settings_address();
    let program_data_pubkey = get_program_data_address();
    let relay_round_pubkey = get_associated_relay_round_address(round);

    let data = RoundLoaderInstruction::Initialize { round, round_ttl }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

pub fn create_proposal(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    current_round_pubkey: &Pubkey,
    round: u32,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(creator_pubkey, round);
    let setting_pubkey = get_associated_settings_address();

    let data = RoundLoaderInstruction::CreateProposal { round }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(setting_pubkey, false),
            AccountMeta::new_readonly(*current_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

pub fn write_proposal(
    creator_pubkey: &Pubkey,
    round: u32,
    offset: u32,
    bytes: Vec<u8>,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(creator_pubkey, round);

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
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
        ],
        data,
    }
}

pub fn finalize_proposal(
    creator_pubkey: &Pubkey,
    current_round_pubkey: &Pubkey,
    round: u32,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(creator_pubkey, round);
    let setting_pubkey = get_associated_settings_address();

    let data = RoundLoaderInstruction::FinalizeProposal { round }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(setting_pubkey, false),
            AccountMeta::new_readonly(*current_round_pubkey, false),
        ],
        data,
    }
}

pub fn vote_for_proposal(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    voter_pubkey: &Pubkey,
    current_round_pubkey: &Pubkey,
    new_round_pubkey: &Pubkey,
    round: u32,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(creator_pubkey, round);
    let setting_pubkey = get_associated_settings_address();

    let data = RoundLoaderInstruction::Vote.try_to_vec().expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*voter_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(*new_round_pubkey, false),
            AccountMeta::new_readonly(*current_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}
