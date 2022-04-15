use borsh::BorshSerialize;
use bridge_utils::{EverAddress, Vote};
use ton_types::UInt256;

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{bpf_loader_upgradeable, system_program, sysvar};
use spl_associated_token_account::get_associated_token_address;

use crate::{id, TokenProxyInstruction};

#[allow(dead_code)]
pub fn get_associated_vault_address(name: &str) -> Pubkey {
    let program_id = &id();
    bridge_utils::get_associated_vault_address(program_id, name)
}

#[allow(dead_code)]
pub fn get_associated_mint_address(name: &str) -> Pubkey {
    let program_id = &id();
    bridge_utils::get_associated_mint_address(program_id, name)
}

#[allow(dead_code)]
pub fn get_associated_settings_address(name: &str) -> Pubkey {
    let program_id = &id();
    bridge_utils::get_associated_settings_address(program_id, Some(name))
}

#[allow(dead_code)]
pub fn get_associated_deposit_address(deposit_seed: u64) -> Pubkey {
    Pubkey::find_program_address(&[br"deposit", &deposit_seed.to_le_bytes()], &id()).0
}

#[allow(dead_code)]
pub fn get_associated_withdrawal_address(
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"withdrawal",
            event_configuration.as_slice(),
            &event_transaction_lt.to_le_bytes(),
        ],
        &id(),
    )
    .0
}

#[allow(dead_code)]
pub fn get_program_data_address() -> Pubkey {
    Pubkey::find_program_address(&[id().as_ref()], &bpf_loader_upgradeable::id()).0
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn initialize_mint(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    name: String,
    decimals: u8,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: Pubkey,
) -> Instruction {
    let program_id = &id();

    let mint_pubkey = bridge_utils::get_associated_mint_address(program_id, &name);
    let settings_pubkey = bridge_utils::get_associated_settings_address(program_id, Some(&name));
    let program_data_pubkey = bridge_utils::get_program_data_address(program_id);

    let data = TokenProxyInstruction::InitializeMint {
        name,
        decimals,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn initialize_vault(
    funder_pubkey: &Pubkey,
    initializer_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    name: String,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
    admin: Pubkey,
) -> Instruction {
    let program_id = &id();

    let vault_pubkey = bridge_utils::get_associated_vault_address(program_id, &name);
    let settings_pubkey = bridge_utils::get_associated_settings_address(program_id, Some(&name));
    let program_data_pubkey = bridge_utils::get_program_data_address(program_id);

    let data = TokenProxyInstruction::InitializeVault {
        name,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
        admin,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*initializer_pubkey, true),
            AccountMeta::new(*mint_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new_readonly(program_data_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn deposit_ever(
    funder_pubkey: &Pubkey,
    sender_pubkey: &Pubkey,
    name: String,
    recipient: EverAddress,
    amount: u64,
    deposit_seed: u64,
) -> Instruction {
    let mint_pubkey = get_associated_mint_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);
    let sender_token_pubkey = get_associated_token_address(sender_pubkey, &mint_pubkey);
    let deposit_pubkey = get_associated_deposit_address(deposit_seed);

    let data = TokenProxyInstruction::DepositEver {
        name,
        recipient,
        amount,
        deposit_seed,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*sender_pubkey, true),
            AccountMeta::new(sender_token_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn deposit_sol(
    funder_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    sender_pubkey: &Pubkey,
    name: String,
    recipient: EverAddress,
    amount: u64,
    deposit_seed: u64,
) -> Instruction {
    let vault_pubkey = get_associated_vault_address(&name);
    let settings_pubkey = get_associated_settings_address(&name);

    let deposit_pubkey = get_associated_deposit_address(deposit_seed);
    let sender_token_pubkey = get_associated_token_address(sender_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::DepositSol {
        name,
        recipient,
        amount,
        deposit_seed,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*sender_pubkey, true),
            AccountMeta::new(sender_token_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(*mint_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn withdrawal_request(
    funder_pubkey: &Pubkey,
    author_pubkey: &Pubkey,
    name: String,
    round_number: u32,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
    sender_address: EverAddress,
    recipient_address: Pubkey,
    amount: u64,
) -> Instruction {
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);
    let relay_round_pubkey =
        bridge_utils::get_associated_relay_round_address(&round_loader::id(), round_number);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::WithdrawRequest {
        name,
        round_number,
        event_configuration,
        event_transaction_lt,
        sender_address,
        recipient_address,
        amount,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*author_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn vote_for_withdrawal_request(
    relay_pubkey: &Pubkey,
    round_number: u32,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
    vote: Vote,
) -> Instruction {
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);
    let relay_round_pubkey =
        bridge_utils::get_associated_relay_round_address(&round_loader::id(), round_number);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::VoteForWithdrawRequest {
        event_configuration,
        event_transaction_lt,
        round_number,
        vote,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*relay_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(relay_round_pubkey, false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn update_withdrawal_status(
    name: String,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::UpdateWithdrawStatus {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(settings_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn withdrawal_ever(
    to_pubkey: &Pubkey,
    name: String,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let mint_pubkey = get_associated_mint_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::WithdrawEver {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn withdrawal_sol(
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let vault_account = get_associated_vault_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::WithdrawSol {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_account, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn approve_withdrawal_ever(
    authority_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let mint_pubkey = get_associated_mint_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, &mint_pubkey);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::ApproveWithdrawEver {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn approve_withdrawal_sol(
    authority_pubkey: &Pubkey,
    name: String,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::ApproveWithdrawSol {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn cancel_withdrawal_sol(
    funder_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
    deposit_seed: u64,
) -> Instruction {
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);
    let deposit_pubkey = get_associated_deposit_address(deposit_seed);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::CancelWithdrawSol {
        event_configuration,
        event_transaction_lt,
        deposit_seed,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(deposit_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn force_withdrawal_sol(
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let vault_account = get_associated_vault_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::ForceWithdrawSol {
        event_configuration,
        event_transaction_lt,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(vault_account, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn fill_withdrawal_sol(
    funder_pubkey: &Pubkey,
    authority_sender_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
    deposit_seed: u64,
    recipient: EverAddress,
) -> Instruction {
    let sender_pubkey = spl_associated_token_account::get_associated_token_address(
        authority_sender_pubkey,
        mint_pubkey,
    );
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);
    let new_deposit_pubkey = get_associated_deposit_address(deposit_seed);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::FillWithdrawSol {
        event_configuration,
        event_transaction_lt,
        deposit_seed,
        recipient,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funder_pubkey, true),
            AccountMeta::new(*authority_sender_pubkey, true),
            AccountMeta::new(sender_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new(withdrawal_pubkey, false),
            AccountMeta::new(new_deposit_pubkey, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn transfer_from_vault(
    authority_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    name: String,
    amount: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);

    let vault_pubkey = get_associated_vault_address(&name);
    let recipient_token_pubkey =
        spl_associated_token_account::get_associated_token_address(to_pubkey, mint_pubkey);

    let data = TokenProxyInstruction::TransferFromVault { name, amount }
        .try_to_vec()
        .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(recipient_token_pubkey, false),
            AccountMeta::new_readonly(settings_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn change_bounty_for_withdrawal_sol(
    authority_pubkey: &Pubkey,
    event_configuration: &UInt256,
    event_transaction_lt: u64,
    bounty: u64,
) -> Instruction {
    let withdrawal_pubkey =
        get_associated_withdrawal_address(event_configuration, event_transaction_lt);

    let event_configuration = bridge_utils::UInt256::from(event_configuration.as_slice());
    let data = TokenProxyInstruction::ChangeBountyForWithdrawSol {
        event_configuration,
        event_transaction_lt,
        bounty,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(withdrawal_pubkey, false),
        ],
        data,
    }
}

#[allow(dead_code)]
pub fn change_settings(
    authority_pubkey: &Pubkey,
    name: String,
    emergency: bool,
    deposit_limit: u64,
    withdrawal_limit: u64,
    withdrawal_daily_limit: u64,
) -> Instruction {
    let settings_pubkey = get_associated_settings_address(&name);

    let data = TokenProxyInstruction::ChangeSettings {
        name,
        emergency,
        deposit_limit,
        withdrawal_limit,
        withdrawal_daily_limit,
    }
    .try_to_vec()
    .expect("pack");

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*authority_pubkey, true),
            AccountMeta::new(settings_pubkey, false),
        ],
        data,
    }
}
