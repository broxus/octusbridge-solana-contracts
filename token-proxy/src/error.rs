use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum TokenProxyError {
    #[error("Deposit limit exceeded")]
    DepositLimit,
    #[error("Invalid token kind")]
    InvalidTokenKind,
    #[error("Relay round expired")]
    RelayRoundExpired,
    #[error("Invalid relay round")]
    InvalidRelayRound,
}

impl From<TokenProxyError> for ProgramError {
    fn from(e: TokenProxyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
