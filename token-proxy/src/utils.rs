use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_associated_settings_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"settings", name.as_bytes()], program_id).0
}

pub fn get_associated_vault_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"vault", name.as_bytes()], program_id).0
}

pub fn get_associated_mint_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&[br"mint", name.as_bytes()], program_id).0
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
