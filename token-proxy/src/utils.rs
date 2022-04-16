use bridge_utils::types::UInt256;

use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_program_data_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
}

pub fn get_associated_settings_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"settings", name.as_bytes()], program_id).0
}

pub fn get_associated_vault_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"vault", name.as_bytes()], program_id).0
}

pub fn get_associated_mint_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"mint", name.as_bytes()], program_id).0
}

pub fn get_associated_deposit_address(program_id: &Pubkey, deposit_seed: u64) -> Pubkey {
    Pubkey::find_program_address(&[br"deposit", &deposit_seed.to_le_bytes()], program_id).0
}

pub fn get_associated_proposal_address(
    program_id: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"proposal",
            event_configuration.as_slice(),
            &event_transaction_lt.to_le_bytes(),
        ],
        program_id,
    )
    .0
}

pub fn validate_programdata_account(
    program_id: &Pubkey,
    programdata_account: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, nonce) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id());

    if pda != *programdata_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

pub fn validate_settings_account(
    program_id: &Pubkey,
    name: &str,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) =
        Pubkey::find_program_address(&[br"settings", name.as_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}

pub fn validate_proposal_account(
    program_id: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            event_configuration.as_slice(),
            &event_transaction_lt.to_le_bytes(),
        ],
        program_id,
    );

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}

pub fn validate_mint_account(
    program_id: &Pubkey,
    name: &str,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[br"mint", name.as_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}

pub fn validate_vault_account(
    program_id: &Pubkey,
    name: &str,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[br"vault", name.as_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}

pub fn validate_deposit_account(
    program_id: &Pubkey,
    deposit_seed: u64,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) =
        Pubkey::find_program_address(&[br"deposit", &deposit_seed.to_le_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}

pub fn validate_initializer_account(
    initializer_account: &Pubkey,
    programdata_account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let upgrade_authority_address = match bincode::deserialize::<UpgradeableLoaderState>(
        &programdata_account_info.data.borrow(),
    )
    .unwrap()
    {
        UpgradeableLoaderState::ProgramData {
            upgrade_authority_address,
            ..
        } => upgrade_authority_address,
        _ => None,
    };

    if upgrade_authority_address.unwrap() != *initializer_account {
        return Err(ProgramError::IllegalOwner);
    }

    Ok(())
}
