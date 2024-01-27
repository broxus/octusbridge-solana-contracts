mod instruction;
mod processor;

pub use self::instruction::*;
pub use self::processor::*;

#[cfg(feature = "bindings")]
mod bindings;

#[cfg(feature = "bindings")]
pub use self::bindings::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("WrapR8ncp6aGqux2TACyJh4MUxcHAHTW9eYzzeXuTJA");
