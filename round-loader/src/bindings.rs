use borsh::BorshSerialize;
use bridge_utils::Vote;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{bpf_loader_upgradeable, system_program, sysvar};
use ton_types::UInt256;

use crate::{id, RoundLoaderInstruction};

pub fn get_associated_settings_address() -> Pubkey {
    Pubkey::find_program_address(&[b"settings"], &id()).0
}

pub fn get_program_data_address() -> Pubkey {
    Pubkey::find_program_address(&[id().as_ref()], &bpf_loader_upgradeable::id()).0
}

pub fn get_associated_relay_round_address(round_number: u32) -> Pubkey {
    Pubkey::find_program_address(&[&round_number.to_le_bytes()], &id()).0
}

pub fn get_associated_proposal_address(
    event_configuration: UInt256,
    event_transaction_lt: u64,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"proposal",
            event_configuration.as_slice(),
            &event_transaction_lt.to_le_bytes(),
        ],
        &id(),
    )
    .0
}

pub fn initialize(
    funder_pubkey: &Pubkey,
    creator_pubkey: &Pubkey,
    round_number: u32,
    round_end: u32,
) -> Instruction {
    let setting_pubkey = get_associated_settings_address();
    let program_data_pubkey = get_program_data_address();
    let relay_round_pubkey = get_associated_relay_round_address(round_number);

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
    let proposal_pubkey =
        get_associated_proposal_address(event_configuration, event_transaction_lt);

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
    let proposal_pubkey =
        get_associated_proposal_address(event_configuration, event_transaction_lt);

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
    let proposal_pubkey =
        get_associated_proposal_address(event_configuration, event_transaction_lt);
    let setting_pubkey = get_associated_settings_address();
    let relay_round_pubkey = get_associated_relay_round_address(round_number);

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
    let proposal_pubkey =
        get_associated_proposal_address(event_configuration, event_transaction_lt);
    let relay_round_pubkey = get_associated_relay_round_address(round_number);

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
    let setting_pubkey = get_associated_settings_address();

    let proposal_pubkey =
        get_associated_proposal_address(event_configuration, event_transaction_lt);
    let relay_round_pubkey = get_associated_relay_round_address(round_number);

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
