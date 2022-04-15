use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::id;

pub fn validate_relay_round_account(
    round_number: u32,
    account_info: &AccountInfo,
) -> Result<u8, ProgramError> {
    let (account, nonce) = Pubkey::find_program_address(&[&round_number.to_le_bytes()], &id());

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(nonce)
}
