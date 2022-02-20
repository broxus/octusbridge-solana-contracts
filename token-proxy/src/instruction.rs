use crate::TokenKind;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TokenProxyInstruction {
    /// Initialize the first round
    ///
    /// # Account references
    ///   0. [WRITE, SIGNER]    Funder account
    ///   1. [WRITE, SIGNER]    Creator account
    ///   2. [WRITE]            Settings account
    ///   3. []                 Buffer Program account
    ///   4. []                 System program
    Initialize {
        /// Token name
        name: String,
        /// Token kind
        kind: TokenKind,
        /// Withdrawals limit
        withdrawal_limit: u64,
        /// Vault deposit limit
        deposit_limit: u64,
        /// Number of base 10 digits to the right of the decimal place.
        decimals: u8,
        /// Admin account
        admin: Pubkey,
        /// Token account
        token: Pubkey,
    },
}
