use borsh::BorshSerialize;
use bridge_utils::types::EverAddress;
use solana_program::account_info::AccountInfo;
use solana_program::hash::hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_associated_settings_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[br"settings"], program_id).0
}

pub fn get_associated_multivault_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[br"multivault"], program_id).0
}

pub fn get_associated_token_settings_ever_address(
    program_id: &Pubkey,
    token: &EverAddress,
) -> Pubkey {
    let token_hash = hash(&token.try_to_vec().expect("pack"));
    Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], program_id).0
}

pub fn get_associated_token_settings_sol_address(program_id: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[br"settings", &mint.to_bytes()], program_id).0
}

pub fn get_associated_mint_address(program_id: &Pubkey, token: &EverAddress) -> Pubkey {
    let token_hash = hash(&token.try_to_vec().expect("pack"));
    Pubkey::find_program_address(&[br"mint", token_hash.as_ref()], program_id).0
}

pub fn get_associated_vault_address(program_id: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[br"vault", &mint.to_bytes()], program_id).0
}

pub fn get_associated_deposit_address(
    program_id: &Pubkey,
    seed: u128,
    token_settings_address: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"deposit",
            &seed.to_le_bytes(),
            &token_settings_address.to_bytes(),
        ],
        program_id,
    )
    .0
}

pub fn validate_token_settings_ever_account(
    program_id: &Pubkey,
    token: &EverAddress,
    nonce: u8,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let token_hash = hash(&token.try_to_vec().expect("pack"));

    let (account, expected_nonce) =
        Pubkey::find_program_address(&[br"settings", token_hash.as_ref()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub fn validate_token_settings_sol_account(
    program_id: &Pubkey,
    mint: &Pubkey,
    nonce: u8,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let (account, expected_nonce) =
        Pubkey::find_program_address(&[br"settings", &mint.to_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub fn validate_mint_account(
    program_id: &Pubkey,
    token: &EverAddress,
    nonce: u8,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let token_hash = hash(&token.try_to_vec().expect("pack"));

    let (account, expected_nonce) =
        Pubkey::find_program_address(&[br"mint", token_hash.as_ref()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub fn validate_vault_account(
    program_id: &Pubkey,
    mint: &Pubkey,
    nonce: u8,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let (account, expected_nonce) =
        Pubkey::find_program_address(&[br"vault", &mint.to_bytes()], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub fn validate_multi_vault_account(
    program_id: &Pubkey,
    nonce: u8,
    account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let (account, expected_nonce) = Pubkey::find_program_address(&[br"multivault"], program_id);

    if account != *account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}
