use std::str::FromStr;
use std::{cmp, fmt, mem};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RELAY_REPARATION: u64 = 20000;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
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

impl FromStr for EverAddress {
    type Err = Box<dyn std::error::Error>;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = string.split(':').take(4).collect();
        let len = parts.len();
        if len != 2 {
            return Err("wrong format".to_string().into());
        }

        let workchain_id = parts[len - 2].parse::<i8>()?;
        let address = hex::decode(parts[len - 1])?;

        Ok(EverAddress::with_standart(
            workchain_id,
            <[u8; 32]>::try_from(address.as_slice())?,
        ))
    }
}

impl fmt::Display for EverAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EverAddress::AddrStd(addr) => {
                write!(f, "{}:{}", addr.workchain_id, hex::encode(addr.address))
            }
        }
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

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Default,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
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

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Default,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct UInt256([u8; 32]);

impl UInt256 {
    pub fn new(value_vec: &[u8]) -> Self {
        Self(
            <[u8; 32]>::try_from(<&[u8]>::clone(&value_vec))
                .expect("Slice must be the same length as a UInt256"),
        )
    }

    pub fn from_be_bytes(value: &[u8]) -> Self {
        let mut data = [0; 32];
        let len = cmp::min(value.len(), 32);
        let offset = 32 - len;
        (0..len).for_each(|i| data[i + offset] = value[i]);
        Self(data)
    }
}

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

impl FromStr for UInt256 {
    type Err = ParseUInt256Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec = hex::decode(s).map_err(|_| ParseUInt256Error::Invalid)?;

        if vec.len() != mem::size_of::<UInt256>() {
            Err(ParseUInt256Error::WrongSize)
        } else {
            Ok(UInt256::new(&vec))
        }
    }
}

#[derive(Error, Debug, Serialize, Clone, PartialEq)]
pub enum ParseUInt256Error {
    #[error("String is the wrong size")]
    WrongSize,
    #[error("Invalid HEX string")]
    Invalid,
}
