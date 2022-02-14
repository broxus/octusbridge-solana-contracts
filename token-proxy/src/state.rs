use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::{Pubkey, PUBKEY_BYTES};

use crate::{utils, TokenKind};

#[derive(Debug)]
pub struct Settings {
    pub is_initialized: bool,
    pub name: String,
    pub kind: TokenKind,
    pub withdrawal_limit: u64,
    pub deposit_limit: u64,
    pub decimals: u8,
    pub admin: Pubkey,
    pub token: Pubkey,
}

impl Sealed for Settings {}

impl IsInitialized for Settings {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

const SETTINGS_LEN: usize = 1 // is_initialized
    + 32  // name TODO! check name size
    + 1 // kind
    + 8 // withdrawal_limit
    + 8 // deposit_limit
    + 1 // decimals
    + PUBKEY_BYTES // admin account address
    + PUBKEY_BYTES // token account address
;

impl Pack for Settings {
    const LEN: usize = SETTINGS_LEN;
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, SETTINGS_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, name, kind, withdrawal_limit, deposit_limit, decimals, admin, token) =
            mut_array_refs![dst, 1, 32, 1, 8, 8, 1, PUBKEY_BYTES, PUBKEY_BYTES];

        utils::pack_bool(self.is_initialized, is_initialized);
        utils::pack_token_kind(self.kind, kind);
        let mut name_str = format!("{:<32}", self.name);
        name_str.truncate(32);
        *name = name_str.into_bytes().try_into().unwrap();
        *withdrawal_limit = self.withdrawal_limit.to_le_bytes();
        *deposit_limit = self.deposit_limit.to_le_bytes();
        *decimals = self.decimals.to_le_bytes();
        *admin = self.admin.to_bytes();
        *token = self.token.to_bytes();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![src, 0, SETTINGS_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, name, kind, withdrawal_limit, deposit_limit, decimals, admin, token) =
            array_refs![input, 1, 32, 1, 8, 8, 1, PUBKEY_BYTES, PUBKEY_BYTES];

        let is_initialized = utils::unpack_bool(is_initialized)?;
        let kind = utils::unpack_token_kind(kind)?;

        let name = String::from_utf8(name.to_vec()).map_err(|_|ProgramError::InvalidAccountData)?;
        let withdrawal_limit = u64::from_le_bytes(*withdrawal_limit);
        let deposit_limit = u64::from_le_bytes(*deposit_limit);
        let decimals = u8::from_le_bytes(*decimals);
        let admin = Pubkey::new_from_array(*admin);
        let token = Pubkey::new_from_array(*token);

        Ok(Self {
            is_initialized,
            kind,
            name,
            withdrawal_limit,
            deposit_limit,
            decimals,
            admin,
            token,
        })
    }
}
