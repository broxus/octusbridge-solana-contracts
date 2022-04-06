use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum EverAddress {
    AddrStd(MsgAddrStd),
}

#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct MsgAddrStd {
    pub workchain_id: i8,
    pub address: [u8; 32],
}
