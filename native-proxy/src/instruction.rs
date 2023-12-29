use borsh::{BorshDeserialize, BorshSerialize};
use bridge_utils::types::{EverAddress, UInt256};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum NativeProxyInstruction {
    Deposit {
        // Deposit seed
        deposit_seed: u128,
        // Deposit amount
        amount: u64,
        // Ever recipient address
        recipient: EverAddress,
        // Sol amount to transfer to ever
        value: u64,
        // Expected SOL amount in EVER
        expected_evers: UInt256,
        // Random payload to transfer to ever
        payload: Vec<u8>,
    },
}
