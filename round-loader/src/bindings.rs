use borsh::BorshSerialize;
use bridge_utils::types::Vote;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

use crate::*;

pub fn get_programdata_address() -> Pubkey {
    let program_id = &id();
    bridge_utils::helper::get_programdata_address(program_id)
}

pub fn get_settings_address() -> Pubkey {
    let program_id = &id();
    get_associated_settings_address(program_id)
}

pub fn get_proposal_address(seed: u64, settings_address: &Pubkey) -> Pubkey {
    let program_id = &id();
    bridge_utils::helper::get_associated_proposal_address(program_id, seed, settings_address)
}

pub fn get_relay_round_address(round_number: u32) -> Pubkey {
    let program_id = &id();
    get_associated_relay_round_address(program_id, round_number)
}

pub fn initialize_ix(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    round_number: u32,
    round_end: u32,
) -> Instruction {
    let program_id = &id();

    let setting_pubkey = get_associated_settings_address(program_id);
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);
    let program_data_pubkey = bridge_utils::helper::get_programdata_address(program_id);

    let data = RoundLoaderInstruction::Initialize {
        round_number,
        round_end,
    }
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
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn create_proposal_ix(funder_pubkey: &Pubkey, proposal_seed: u64) -> Instruction {
    let program_id = &id();

    let settings_address = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        proposal_seed,
        &settings_address,
    );

    let data = RoundLoaderInstruction::CreateProposal { proposal_seed }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn write_proposal_ix(proposal_seed: u64, offset: u32, bytes: Vec<u8>) -> Instruction {
    let program_id = &id();

    let settings_address = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        proposal_seed,
        &settings_address,
    );

    let data = RoundLoaderInstruction::WriteProposal {
        proposal_seed,
        offset,
        bytes,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![AccountMeta::new(proposal_pubkey, false)],
        data,
    }
}

pub fn finalize_proposal_ix(
    creator_pubkey: &Pubkey,
    proposal_seed: u64,
    round_number: u32,
) -> Instruction {
    let program_id = &id();

    let settings_address = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        proposal_seed,
        &settings_address,
    );
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

    let data = RoundLoaderInstruction::FinalizeProposal { proposal_seed }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(settings_address, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn vote_for_proposal_ix(
    voter_pubkey: &Pubkey,
    proposal_seed: u64,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let program_id = &id();

    let settings_address = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        proposal_seed,
        &settings_address,
    );
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

    let data = RoundLoaderInstruction::VoteForProposal {
        proposal_seed,
        settings_address,
        vote,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*voter_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    }
}

pub fn execute_proposal_ix(
    funder_pubkey: &Pubkey,
    proposal_seed: u64,
    round_number: u32,
) -> Instruction {
    let program_id = &id();

    let settings_address = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        proposal_seed,
        &settings_address,
    );
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

    let data = RoundLoaderInstruction::ExecuteProposal
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(settings_address, false),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}
