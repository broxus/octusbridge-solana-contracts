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

#[derive(Copy)]
pub enum TokenKind {
    Ever = 0,
    Solana = 1,
}

pub fn pack_token_kind(kind: TokenKind, dst: &mut [u8; 1]) {
    *dst = (kind as u8).to_le_bytes()
}

pub fn unpack_token_kind(src: &[u8; 1]) -> Result<TokenKind, ProgramError> {
    match u8::from_le_bytes(*src) {
        0 => Ok(TokenKind::Ever),
        1 => Ok(TokenKind::Solana),
        _ => Err(ProgramError::InvalidAccountData),
    }
}
