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
    #[error("Relay not in the round")]
    InvalidRelay,
    #[error("Emergency mode enabled")]
    EmergencyEnabled,
    #[error("Invalid withdrawal status")]
    InvalidWithdrawalStatus,
    #[error("Insufficient vault balance")]
    InsufficientVaultBalance,
    #[error("Relay already voted")]
    RelayAlreadyVoted,
    #[error("Arithmetics error")]
    ArithmeticsError,
    #[error("Token name is too long")]
    TokenNameLenLimit,
    #[error("Invalid vote")]
    InvalidVote,
    #[error("Votes overflow")]
    VotesOverflow,
}

impl From<TokenProxyError> for ProgramError {
    fn from(e: TokenProxyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
