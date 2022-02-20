use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::{bpf_loader_upgradeable, system_instruction};

pub fn fund_account<'a>(
    account_info: &AccountInfo<'a>,
    funder_account_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    data_len: usize,
) -> Result<(), ProgramError> {
    let required_lamports = Rent::default()
        .minimum_balance(data_len)
        .max(1)
        .saturating_sub(account_info.lamports());

    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(
                funder_account_info.key,
                account_info.key,
                required_lamports,
            ),
            &[
                funder_account_info.clone(),
                account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    Ok(())
}

pub fn create_account<'a>(
    program_id: &Pubkey,
    account_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    account_signer_seeds: &[&[u8]],
    space: usize,
) -> Result<(), ProgramError> {
    invoke_signed(
        &system_instruction::allocate(account_info.key, space as u64),
        &[account_info.clone(), system_program_info.clone()],
        &[account_signer_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(account_info.key, program_id),
        &[account_info.clone(), system_program_info.clone()],
        &[account_signer_seeds],
    )
}

pub fn validate_programdata_account(
    program_id: &Pubkey,
    programdata_account: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, nonce) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id());

    if pda != *programdata_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

pub fn validate_creator_account(
    creator_account: &Pubkey,
    programdata_account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let upgrade_authority_address = match bincode::deserialize::<UpgradeableLoaderState>(
        &programdata_account_info.data.borrow(),
    )
    .unwrap()
    {
        UpgradeableLoaderState::ProgramData {
            upgrade_authority_address,
            ..
        } => upgrade_authority_address,
        _ => None,
    };

    if upgrade_authority_address.unwrap() != *creator_account {
        return Err(ProgramError::IllegalOwner);
    }

    Ok(())
}

pub fn validate_settings_account(
    program_id: &Pubkey,
    settings_account: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(&[b"settings"], program_id);

    if pda != *settings_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

pub fn validate_round_relay_account(
    program_id: &Pubkey,
    round_relay_account: &Pubkey,
    round: u32,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(&[&round.to_le_bytes()], program_id);

    if pda != *round_relay_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}

pub fn validate_proposal_account(
    program_id: &Pubkey,
    creator_account: &Pubkey,
    proposal_account: &Pubkey,
    round: u32,
) -> Result<u8, ProgramError> {
    let (pda, nonce) = Pubkey::find_program_address(
        &[&creator_account.to_bytes(), &round.to_le_bytes()],
        program_id,
    );
    if pda != *proposal_account {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(nonce)
}
