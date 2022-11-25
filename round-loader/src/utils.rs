use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn validate_relay_round_account(
    program_id: &Pubkey,
    round_number: u32,
    nonce: u8,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let (account, expected_nonce) =
        Pubkey::find_program_address(&[br"relay_round", &round_number.to_le_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}
