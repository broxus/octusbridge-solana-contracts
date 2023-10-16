use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::Vote;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct VoteForProposal {
    // Instruction number
    pub instruction: u8,
    // Vote type
    pub vote: Vote,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecuteProposal {
    // Instruction number
    pub instruction: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutePayload {
    // Instruction number
    pub instruction: u8,
}

pub fn vote_for_proposal_ix(
    program_id: Pubkey,
    instruction: u8,
    voter_pubkey: &Pubkey,
    proposal_pubkey: &Pubkey,
    round_number: u32,
    vote: Vote,
) -> Instruction {
    let relay_round_pubkey = round_loader::get_relay_round_address(round_number);

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
    proposal_pubkey: Pubkey,
    accounts: Vec<(Pubkey, bool, bool)>,
) -> Instruction {
    let data = ExecuteProposal { instruction }.try_to_vec().expect("pack");

    let mut accounts_with_meta = Vec::with_capacity(accounts.len() + 1);
    accounts_with_meta.push((proposal_pubkey, false, false));
    accounts_with_meta.extend(accounts);

    let accounts = accounts_with_meta
        .into_iter()
        .map(|(account, read_only, is_signer)| {
            if !read_only {
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

pub fn execute_payload_ix(
    program_id: Pubkey,
    instruction: u8,
    proposal_pubkey: Pubkey,
    accounts: Vec<(Pubkey, bool, bool)>,
) -> Instruction {
    let data = ExecutePayload { instruction }.try_to_vec().expect("pack");

    let mut accounts_with_meta = Vec::with_capacity(accounts.len() + 1);
    accounts_with_meta.push((proposal_pubkey, false, false));
    accounts_with_meta.extend(accounts);

    let accounts = accounts_with_meta
        .into_iter()
        .map(|(account, read_only, is_signer)| {
            if !read_only {
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

#[allow(clippy::too_many_arguments)]
pub fn withdrawal_ever_request_ix(
    program_id: Pubkey,
    creator_pubkey: Pubkey,
    withdrawal_pubkey: Pubkey,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    event: token_proxy::WithdrawalMultiTokenEverEvent,
    attached_amount: u64,
) -> Instruction {
    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());
    let relay_round_pubkey =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let mut accounts = vec![
        AccountMeta::new(creator_pubkey, true),
        AccountMeta::new(creator_pubkey, true),
        AccountMeta::new(withdrawal_pubkey, false),
        AccountMeta::new_readonly(rl_settings_pubkey, false),
        AccountMeta::new_readonly(relay_round_pubkey, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    if !event.payload.is_empty() {
        let mint = token_proxy::get_associated_mint_address(&program_id, &event.token);

        let proxy_pubkey =
            token_proxy::get_associated_proxy_address(&program_id, &mint, &event.recipient);
        accounts.push(AccountMeta::new(proxy_pubkey, false));
    }

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenEverRequest {
        attached_amount,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        token: event.token,
        name: event.name,
        symbol: event.symbol,
        decimals: event.decimals,
        recipient: event.recipient,
        amount: event.amount,
        payload: event.payload,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id,
        accounts,
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn withdrawal_sol_request_ix(
    program_id: Pubkey,
    creator_pubkey: Pubkey,
    withdrawal_pubkey: Pubkey,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: Pubkey,
    event: token_proxy::WithdrawalMultiTokenSolEvent,
    attached_amount: u64,
) -> Instruction {
    let token_settings_pubkey =
        token_proxy::get_associated_token_settings_sol_address(&program_id, &event.mint);

    let rl_settings_pubkey =
        bridge_utils::helper::get_associated_settings_address(&round_loader::id());
    let relay_round_pubkey =
        bridge_utils::helper::get_associated_relay_round_address(&round_loader::id(), round_number);

    let mut accounts = vec![
        AccountMeta::new(creator_pubkey, true),
        AccountMeta::new(creator_pubkey, true),
        AccountMeta::new(withdrawal_pubkey, false),
        AccountMeta::new_readonly(token_settings_pubkey, false),
        AccountMeta::new_readonly(rl_settings_pubkey, false),
        AccountMeta::new_readonly(relay_round_pubkey, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    if !event.payload.is_empty() {
        let proxy_pubkey =
            token_proxy::get_associated_proxy_address(&program_id, &event.mint, &event.recipient);
        accounts.push(AccountMeta::new(proxy_pubkey, false));
    }

    let data = token_proxy::TokenProxyInstruction::WithdrawMultiTokenSolRequest {
        attached_amount,
        event_timestamp,
        event_transaction_lt,
        event_configuration,
        recipient: event.recipient,
        amount: event.amount,
        payload: event.payload,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id,
        accounts,
        data,
    }
}
