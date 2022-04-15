use borsh::BorshSerialize;
use bridge_utils::Vote;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use ton_types::UInt256;

use crate::{id, RoundLoaderInstruction};

pub fn get_associated_settings_address() -> Pubkey {
    let program_id = &id();
    bridge_utils::get_associated_settings_address(program_id, None)
}

pub fn get_program_data_address() -> Pubkey {
    let program_id = &id();
    bridge_utils::get_program_data_address(program_id)
}

pub fn get_associated_relay_round_address(round_number: u32) -> Pubkey {
    let program_id = &id();
    bridge_utils::get_associated_relay_round_address(program_id, round_number)
}

pub fn get_associated_proposal_address(
    event_configuration: UInt256,
    event_transaction_lt: u64,
) -> Pubkey {
    let program_id = &id();
    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());

    bridge_utils::get_associated_proposal_address(
        program_id,
        event_configuration,
        event_transaction_lt,
    )
}

pub fn initialize(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    round_number: u32,
    round_end: u32,
) -> Instruction {
    let program_id = &id();

    let setting_pubkey = bridge_utils::get_associated_settings_address(program_id, None);
    let program_data_pubkey = bridge_utils::get_program_data_address(program_id);
    let relay_round_pubkey =
        bridge_utils::get_associated_relay_round_address(program_id, round_number);

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

pub fn create_proposal(
    funder_pubkey: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let program_id = &id();
    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());

    let proposal_pubkey = bridge_utils::get_associated_proposal_address(
        program_id,
        event_configuration,
        event_transaction_lt,
    );

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = RoundLoaderInstruction::CreateProposal {
        event_configuration,
        event_transaction_lt,
    }
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

pub fn write_proposal(
    event_configuration: UInt256,
    event_transaction_lt: u64,
    offset: u32,
    bytes: Vec<u8>,
) -> Instruction {
    let program_id = &id();
    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());

    let proposal_pubkey = bridge_utils::get_associated_proposal_address(
        program_id,
        event_configuration,
        event_transaction_lt,
    );

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = RoundLoaderInstruction::WriteProposal {
        event_configuration,
        event_transaction_lt,
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

pub fn finalize_proposal(
    creator_pubkey: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
    round_number: u32,
) -> Instruction {
    let program_id = &id();
    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());

    let proposal_pubkey = bridge_utils::get_associated_proposal_address(
        program_id,
        event_configuration,
        event_transaction_lt,
    );
    let setting_pubkey = bridge_utils::get_associated_settings_address(program_id, None);
    let relay_round_pubkey =
        bridge_utils::get_associated_relay_round_address(program_id, round_number);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = RoundLoaderInstruction::FinalizeProposal {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*creator_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(setting_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

pub fn vote_for_proposal(
    voter_pubkey: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let program_id = &id();
    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());

    let proposal_pubkey = bridge_utils::get_associated_proposal_address(
        program_id,
        event_configuration,
        event_transaction_lt,
    );
    let relay_round_pubkey =
        bridge_utils::get_associated_relay_round_address(program_id, round_number);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = RoundLoaderInstruction::VoteForProposal {
        event_configuration,
        event_transaction_lt,
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

pub fn execute_proposal(
    funder_pubkey: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
    round_number: u32,
) -> Instruction {
    let program_id = &id();
    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());

    let setting_pubkey = bridge_utils::get_associated_settings_address(program_id, None);

    let proposal_pubkey = bridge_utils::get_associated_proposal_address(
        program_id,
        event_configuration,
        event_transaction_lt,
    );
    let relay_round_pubkey =
        bridge_utils::get_associated_relay_round_address(program_id, round_number);

    let data = RoundLoaderInstruction::ExecuteProposal
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}
