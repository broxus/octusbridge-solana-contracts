use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_associated_settings_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"settings"], program_id).0
}

pub fn get_associated_relay_round_address(program_id: &Pubkey, round_number: u32) -> Pubkey {
    Pubkey::find_program_address(&[&round_number.to_le_bytes()], program_id).0
}

pub fn validate_settings_account(
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[br"settings"], program_id);

    if account_info.owner != program_id {
        return Err(ProgramError::InvalidArgument);
    }

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}

pub fn validate_relay_round_account(
    program_id: &Pubkey,
    round_number: u32,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[&round_number.to_le_bytes()], program_id);

    if account_info.owner != program_id {
        return Err(ProgramError::InvalidArgument);
    }

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}
