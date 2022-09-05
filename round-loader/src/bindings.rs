use borsh::BorshSerialize;
use bridge_utils::types::Vote;

use solana_program::hash::hash;
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
    bridge_utils::helper::get_associated_settings_address(program_id)
}

pub fn get_proposal_address(
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
    event_data: &[u8],
) -> Pubkey {
    let program_id = &id();

    let settings = &get_settings_address();

    let event_data = hash(event_data);

    bridge_utils::helper::get_associated_proposal_address(
        program_id,
        settings,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        &event_data.to_bytes(),
    )
}

pub fn get_relay_round_address(round_number: u32) -> Pubkey {
    let program_id = &id();
    get_associated_relay_round_address(program_id, round_number)
}

pub fn initialize_ix(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    genesis_round_number: u32,
    round_submitter: Pubkey,
    min_required_votes: u32,
    round_ttl: u32,
) -> Instruction {
    let setting_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = RoundLoaderInstruction::Initialize {
        genesis_round_number,
        round_submitter,
        min_required_votes,
        round_ttl,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn update_settings_ix(
    author_pubkey: &Pubkey,
    current_round_number: Option<u32>,
    round_submitter: Option<Pubkey>,
    min_required_votes: Option<u32>,
    round_ttl: Option<u32>,
) -> Instruction {
    let setting_pubkey = get_settings_address();
    let program_data_pubkey = get_programdata_address();

    let data = RoundLoaderInstruction::UpdateSettings {
        current_round_number,
        round_submitter,
        min_required_votes,
        round_ttl,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
        ],
        data,
    }
}

pub fn create_relay_round_ix(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    round_number: u32,
    round_end: u32,
    relays: Vec<Pubkey>,
) -> Instruction {
    let setting_pubkey = get_settings_address();
    let relay_round_pubkey = get_relay_round_address(round_number);

    let data = RoundLoaderInstruction::CreateRelayRound {
        round_number,
        round_end,
        relays,
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
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn create_proposal_ix(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    event_data: &[u8],
) -> Instruction {
    let proposal_pubkey = get_proposal_address(
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
        event_data,
    );

    let event_data = hash(event_data);

    let data = RoundLoaderInstruction::CreateProposal {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        event_data,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn write_proposal_ix(
    creator_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    offset: u32,
    bytes: Vec<u8>,
) -> Instruction {
    let data = RoundLoaderInstruction::WriteProposal { offset, bytes }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
        ],
        data,
    }
}

pub fn finalize_proposal_ix(
    creator_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    round_number: u32,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let relay_round_pubkey = get_relay_round_address(round_number);

    let data = RoundLoaderInstruction::FinalizeProposal
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

pub fn vote_for_proposal_ix(
    voter_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let relay_round_pubkey = get_relay_round_address(round_number);

    let data = RoundLoaderInstruction::VoteForProposal { vote }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*voter_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    }
}

pub fn execute_proposal_ix(
    funder_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    round_number: u32,
) -> Instruction {
    let settings_pubkey = get_settings_address();
    let relay_round_pubkey = get_relay_round_address(round_number);

    let data = RoundLoaderInstruction::ExecuteProposal
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}
