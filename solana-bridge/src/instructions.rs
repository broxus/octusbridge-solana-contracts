use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::Vote;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct VoteForProposal {
    // Instruction number
    instruction: u8,
    // Vote type
    vote: Vote,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct ExecuteProposal {
    // Instruction number
    instruction: u8,
}

pub fn vote_for_proposal_ix(
    program_id: Pubkey,
    instruction: u8,
    voter_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let relay_round_pubkey =
        round_loader::get_associated_relay_round_address(&round_loader::id(), round_number);

    let data = VoteForProposal { instruction, vote }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*voter_pubkey, true),
            AccountMeta::new(*proposal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    }
}

pub fn execute_proposal_ix(
    program_id: Pubkey,
    instruction: u8,
    accounts: Vec<(Pubkey, bool, bool)>,
) -> Instruction {
    let data = ExecuteProposal { instruction }.try_to_vec().expect("pack");

    let accounts = accounts
        .into_iter()
        .map(|(account, is_writable, is_signer)| {
            if is_writable {
                AccountMeta::new(account, is_signer)
            } else {
                AccountMeta::new_readonly(account, is_signer)
            }
        })
        .collect();

    Instruction {
        program_id,
        accounts,
        data,
    }
}
