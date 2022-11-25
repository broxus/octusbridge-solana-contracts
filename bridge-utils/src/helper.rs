use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_programdata_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
}

pub fn get_associated_settings_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"settings"], program_id).0
}

pub fn get_associated_relay_round_address(program_id: &Pubkey, round_number: u32) -> Pubkey {
    Pubkey::find_program_address(&[br"relay_round", &round_number.to_le_bytes()], program_id).0
}

pub fn get_associated_proposal_address(
    program_id: &Pubkey,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
    event_data: &[u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            event_data,
        ],
        program_id,
    )
    .0
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

pub fn validate_initializer_account(
    initializer_account: &Pubkey,
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

    if upgrade_authority_address.unwrap() != *initializer_account {
        return Err(ProgramError::IllegalOwner);
    }

    Ok(())
}

pub fn validate_proposal_account(
    program_id: &Pubkey,
    round_number: u32,
    event_timestamp: u32,
    event_transaction_lt: u64,
    event_configuration: &Pubkey,
    event_data: &Hash,
    nonce: u8,
    proposal_account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    let (account, expected_nonce) = Pubkey::find_program_address(
        &[
            br"proposal",
            &round_number.to_le_bytes(),
            &event_timestamp.to_le_bytes(),
            &event_transaction_lt.to_le_bytes(),
            &event_configuration.to_bytes(),
            &event_data.to_bytes(),
        ],
        program_id,
    );

    if account != *proposal_account_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    if expected_nonce != nonce {
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}
