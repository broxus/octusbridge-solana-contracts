use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum Vote {
    None,
    Confirm,
    Reject,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub enum EverAddress {
    AddrStd(MsgAddrStd),
}

impl EverAddress {
    pub fn with_standart(workchain_id: i8, address: [u8; 32]) -> Self {
        EverAddress::AddrStd(MsgAddrStd::with_address(workchain_id, address))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct MsgAddrStd {
    pub workchain_id: i8,
    pub address: [u8; 32],
}

impl MsgAddrStd {
    pub fn with_address(workchain_id: i8, address: [u8; 32]) -> Self {
        MsgAddrStd {
            workchain_id,
            address,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, BorshSerialize, BorshDeserialize)]
pub struct UInt128([u8; 16]);

impl From<[u8; 16]> for UInt128 {
    fn from(data: [u8; 16]) -> Self {
        UInt128(data)
    }
}

impl From<&[u8; 16]> for UInt128 {
    fn from(data: &[u8; 16]) -> Self {
        UInt128(*data)
    }
}

impl UInt128 {
    pub const fn as_slice(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, BorshSerialize, BorshDeserialize)]
pub struct UInt256([u8; 32]);

impl From<[u8; 32]> for UInt256 {
    fn from(data: [u8; 32]) -> Self {
        UInt256(data)
    }
}

impl From<&[u8; 32]> for UInt256 {
    fn from(data: &[u8; 32]) -> Self {
        UInt256(*data)
    }
}

impl UInt256 {
    pub const fn as_slice(&self) -> &[u8; 32] {
        &self.0
    }
}
