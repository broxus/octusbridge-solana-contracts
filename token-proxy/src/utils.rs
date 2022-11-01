use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_associated_settings_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[br"settings"], program_id).0
}

pub fn get_associated_token_settings_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"settings", name.as_bytes()], program_id).0
}

pub fn get_associated_vault_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"vault", name.as_bytes()], program_id).0
}

pub fn get_associated_mint_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"mint", name.as_bytes()], program_id).0
}

pub fn get_associated_deposit_address(
    program_id: &Pubkey,
    seed: u128,
    settings_address: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"deposit",
            &seed.to_le_bytes(),
            &settings_address.to_bytes(),
        ],
        program_id,
    )
    .0
}

pub fn validate_token_settings_account(
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

pub fn validate_deposit_account(
    program_id: &Pubkey,
    seed: u128,
    token_settings_address: &Pubkey,
    deposit_account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(
        &[
            br"deposit",
            &seed.to_le_bytes(),
            &token_settings_address.to_bytes(),
        ],
        program_id,
    );

    if account != *deposit_account_info.key {
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

pub fn validate_multi_vault_account(
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[br"multivault"], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}
