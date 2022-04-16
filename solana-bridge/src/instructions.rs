use bridge_utils::types::{UInt256, Vote};

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum BridgeInstruction {
    VoteForProposal {
        // EVER->SOL event configuration
        event_configuration: UInt256,
        // Ever deployed event transaction_lt
        event_transaction_lt: u64,
        // Vote type
        vote: Vote,
    },
}

pub fn vote_for_proposal_ix(
    program_id: Pubkey,
    voter_pubkey: Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        &program_id,
        event_configuration,
        event_transaction_lt,
    );

    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&program_id, round_number);

    let data = BridgeInstruction::VoteForProposal {
        event_configuration,
        event_transaction_lt,
        vote,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(voter_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    }
}
