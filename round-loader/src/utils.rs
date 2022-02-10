use solana_program::program_error::ProgramError;

pub fn pack_bool(boolean: bool, dst: &mut [u8; 1]) {
    *dst = (boolean as u8).to_le_bytes()
}

pub fn unpack_bool(src: &[u8; 1]) -> Result<bool, ProgramError> {
    match u8::from_le_bytes(*src) {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProgramError::InvalidAccountData),
    }
}
