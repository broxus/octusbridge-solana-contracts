use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::Vote;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct VoteForProposal {
    // Instruction number
    instruction: u8,
    // Proposal seed
    proposal_seed: u128,
    // Settings address
    settings_address: Pubkey,
    // Vote type
    vote: Vote,
}

pub fn vote_for_proposal_ix(
    program_id: Pubkey,
    voter_pubkey: Pubkey,
    instruction: u8,
    proposal_seed: u128,
    settings_address: Pubkey,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let proposal_pubkey = bridge_utils::helper::get_associated_proposal_address(
        &program_id,
        proposal_seed,
        &settings_address,
    );

    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&program_id, round_number);

    let data = VoteForProposal {
        instruction,
        proposal_seed,
        settings_address,
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
