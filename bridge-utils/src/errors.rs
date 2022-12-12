use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum SolanaBridgeError {
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
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Insufficient vault balance")]
    InsufficientVaultBalance,
    #[error("Relay already voted")]
    RelayAlreadyVoted,
    #[error("Operation overflowed")]
    Overflow,
    #[error("Token name is too long")]
    TokenNameLenLimit,
    #[error("Token symbol is too long")]
    TokenSymbolLenLimit,
    #[error("Invalid vote")]
    InvalidVote,
    #[error("Votes overflow")]
    VotesOverflow,
    #[error("Invalid token settings name")]
    InvalidTokenSettingsName,
    #[error("Failed to deserialize payload")]
    DeserializePayload,
}

impl From<SolanaBridgeError> for ProgramError {
    fn from(e: SolanaBridgeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl TryFrom<u32> for SolanaBridgeError {
    type Error = ();

    fn try_from(val: u32) -> Result<SolanaBridgeError, ()> {
        match val {
            0 => Ok(SolanaBridgeError::DepositLimit),
            1 => Ok(SolanaBridgeError::InvalidTokenKind),
            2 => Ok(SolanaBridgeError::RelayRoundExpired),
            3 => Ok(SolanaBridgeError::InvalidRelayRound),
            4 => Ok(SolanaBridgeError::InvalidRelay),
            5 => Ok(SolanaBridgeError::EmergencyEnabled),
            6 => Ok(SolanaBridgeError::InvalidWithdrawalStatus),
            7 => Ok(SolanaBridgeError::InsufficientVaultBalance),
            8 => Ok(SolanaBridgeError::RelayAlreadyVoted),
            9 => Ok(SolanaBridgeError::Overflow),
            10 => Ok(SolanaBridgeError::TokenNameLenLimit),
            11 => Ok(SolanaBridgeError::TokenSymbolLenLimit),
            12 => Ok(SolanaBridgeError::InvalidVote),
            13 => Ok(SolanaBridgeError::VotesOverflow),
            _ => Err(()),
        }
    }
}
