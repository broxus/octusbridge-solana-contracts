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

pub fn get_proposal_address(
    author: &Pubkey,
    settings: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
) -> Pubkey {
    let program_id = &id();
    bridge_utils::helper::get_associated_proposal_address(
        program_id,
        author,
        settings,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
    )
}

pub fn get_relay_round_address(round_number: u32) -> Pubkey {
    let program_id = &id();
    get_associated_relay_round_address(program_id, round_number)
}

pub fn initialize_ix(
    initializer_pubkey: &Pubkey,
    round_number: u32,
    round_end: u32,
    relays: Vec<Pubkey>,
) -> Instruction {
    let program_id = &id();

    let setting_pubkey = get_associated_settings_address(program_id);
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);
    let program_data_pubkey = bridge_utils::helper::get_programdata_address(program_id);

    let data = RoundLoaderInstruction::Initialize {
        round_number,
        round_end,
        relays,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

pub fn create_proposal_ix(
    creator_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
) -> Instruction {
    let program_id = &id();

    let settings = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        creator_pubkey,
        &settings,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
    );

    let data = RoundLoaderInstruction::CreateProposal {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
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
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    offset: u32,
    bytes: Vec<u8>,
) -> Instruction {
    let program_id = &id();

    let settings = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        creator_pubkey,
        &settings,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
    );

    let data = RoundLoaderInstruction::WriteProposal {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
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

pub fn finalize_proposal_ix(
    creator_pubkey: &Pubkey,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    round_number: u32,
) -> Instruction {
    let program_id = &id();

    let settings_pubkey = get_associated_settings_address(program_id);

    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        program_id,
        creator_pubkey,
        &settings_pubkey,
        event_timestamp,
        event_transaction_lt,
        &event_configuration,
    );
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

    let data = RoundLoaderInstruction::FinalizeProposal {
        event_timestamp,
        event_transaction_lt,
        event_configuration,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
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
    let program_id = &id();

    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

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
    let program_id = &id();

    let settings_pubkey = get_associated_settings_address(program_id);
    let relay_round_pubkey = get_associated_relay_round_address(program_id, round_number);

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
