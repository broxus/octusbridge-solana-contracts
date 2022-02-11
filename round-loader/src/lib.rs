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

pub fn get_associated_proposal_address(relay_address: &Pubkey, round: u32) -> Pubkey {
    Pubkey::find_program_address(&[&relay_address.to_bytes(), &round.to_le_bytes()], &id()).0
}

pub fn get_associated_settings_address() -> Pubkey {
    Pubkey::find_program_address(&[b"settings"], &id()).0
}

pub fn get_associated_relay_round_address(round: u32) -> Pubkey {
    Pubkey::find_program_address(&[&round.to_le_bytes()], &id()).0
}

/*pub fn initialize(
    authority_pubkey: &Pubkey,
    program_buffer_pubkey: &Pubkey,
    round: u32,
    round_ttl: u32,
) -> Instruction {
    let setting_pubkey = get_associated_settings_address();
    let relay_round_pubkey = get_associated_relay_round_address(round);

    let data = RoundLoaderInstruction::Initialize { round, round_ttl }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(relay_round_pubkey, false),
            AccountMeta::new_readonly(id(), false),
            AccountMeta::new_readonly(*program_buffer_pubkey, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}

pub fn create_proposal(
    relay_pubkey: &Pubkey,
    current_round_pubkey: &Pubkey,
    round: u32,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(relay_pubkey, round);
    let setting_pubkey = get_associated_settings_address();

    let data = RoundLoaderInstruction::CreateProposal { round }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(setting_pubkey, false),
            AccountMeta::new_readonly(*current_round_pubkey, false),
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

pub fn finalize_proposal(
    relay_pubkey: &Pubkey,
    current_round_pubkey: &Pubkey,
    round: u32,
) -> Instruction {
    let proposal_pubkey = get_associated_proposal_address(relay_pubkey, round);
    let setting_pubkey = get_associated_settings_address();

    let data = RoundLoaderInstruction::FinalizeProposal { round }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(setting_pubkey, false),
            AccountMeta::new_readonly(*current_round_pubkey, false),
        ],
        data,
    }
}

pub fn vote_for_proposal(
    relay_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    current_round_pubkey: &Pubkey,
    new_round_account_info: &Pubkey,
) -> Instruction {
    let setting_pubkey = get_associated_settings_address();

    let data = RoundLoaderInstruction::Vote.try_to_vec().expect("pack");

    println!("{}", current_round_pubkey);
    println!("{}", new_round_account_info);

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new(setting_pubkey, false),
            AccountMeta::new(*new_round_account_info, false),
            AccountMeta::new_readonly(*current_round_pubkey, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data,
    }
}*/
