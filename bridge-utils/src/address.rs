use solana_program::bpf_loader_upgradeable;
use solana_program::pubkey::Pubkey;

use crate::UInt256;

pub fn get_associated_settings_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"settings"], program_id).0
}

pub fn get_program_data_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
}

pub fn get_associated_relay_round_address(program_id: &Pubkey, round_number: u32) -> Pubkey {
    Pubkey::find_program_address(&[&round_number.to_le_bytes()], program_id).0
}

pub fn get_associated_proposal_address(
    program_id: &Pubkey,
    event_configuration: UInt256,
    event_transaction_lt: u64,
) -> Pubkey {
    Pubkey::find_program_address(
        &[
            br"proposal",
            event_configuration.as_slice(),
            &event_transaction_lt.to_le_bytes(),
        ],
        program_id,
    )
    .0
}
