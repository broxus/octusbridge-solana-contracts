use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

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

pub fn validate_mint_account(
    program_id: &Pubkey,
    name: &str,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[br"mint", name.as_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidAccountData);
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
        return Err(ProgramError::InvalidAccountData);
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
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(nonce)
}

pub fn validate_deposit_account(
    program_id: &Pubkey,
    payload_id: &Hash,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) =
        Pubkey::find_program_address(&[br"deposit", &payload_id.to_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(nonce)
}

pub fn validate_withdraw_account(
    program_id: &Pubkey,
    payload_id: &Hash,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) =
        Pubkey::find_program_address(&[br"withdrawal", &payload_id.to_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(nonce)
}
