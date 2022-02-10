use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum RoundLoaderError {
    #[error("proposal relays data too small for instruction")]
    ProposalRelaysDataTooSmall,
    #[error("Relay doesn't have permission to vote for proposal")]
    InvalidRelay,
    #[error("Relay already voted for proposal")]
    RelayAlreadyVoted,
}

impl From<RoundLoaderError> for ProgramError {
    fn from(e: RoundLoaderError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
