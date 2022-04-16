use bridge_utils::types::{UInt256, Vote};

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn vote_for_proposal_ix(
    program_id: Pubkey,
    voter_pubkey: Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
    round_number: u32,
    vote: Vote,
) -> Result<Instruction, ProgramError> {
    let (proposal_pubkey, relay_round_pubkey, data) = match program_id {
        &round_loader::id() => {
            let proposal_pubkey = round_loader::get_associated_proposal_address(
                &program_id,
                event_configuration,
                event_transaction_lt,
            );
            let relay_round_pubkey =
                round_loader::get_associated_relay_round_address(&program_id, round_number);

            let data = round_loader::RoundLoaderInstruction::VoteForProposal {
                event_configuration,
                event_transaction_lt,
                vote,
            }
            .try_to_vec()
            .expect("pack");

            (proposal_pubkey, relay_round_pubkey, data)
        }
        &token_proxy::id() => {
            let proposal_pubkey = token_proxy::get_associated_proposal_address(
                &program_id,
                event_configuration,
                event_transaction_lt,
            );
            let relay_round_pubkey =
                round_loader::get_associated_relay_round_address(&program_id, round_number);

            let data = token_proxy::TokenProxyInstruction::VoteForWithdrawRequest {
                event_configuration,
                event_transaction_lt,
                vote,
            }
            .try_to_vec()
            .expect("pack");

            (proposal_pubkey, relay_round_pubkey, data)
        }
        _ => return Err(ProgramError::IncorrectProgramId),
    };

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(voter_pubkey, true),
            AccountMeta::new(proposal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    })
}
