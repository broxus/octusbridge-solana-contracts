use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum TokenProxyError {
    #[error("Relay not in the current round")]
    InvalidRelay,
    #[error("Relay already voted for proposal")]
    RelayAlreadyVoted,
}

impl From<TokenProxyError> for ProgramError {
    fn from(e: TokenProxyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
