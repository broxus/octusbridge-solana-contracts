mod instruction;
mod processor;
mod state;
mod utils;

pub use self::instruction::*;
pub use self::processor::*;
pub use self::state::*;
pub use self::utils::*;

#[cfg(feature = "bindings")]
mod bindings;

#[cfg(feature = "bindings")]
pub use self::bindings::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("octuswa5MD5hrTwcNBKvdxDvDQoz7C7M9sk2cRRvZfg");
