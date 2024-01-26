mod instruction;
mod state;
mod utils;

pub use self::instruction::*;
pub use self::state::*;
pub use self::utils::*;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
extern crate wasm_bindgen;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm;

#[cfg(feature = "bindings")]
mod bindings;

#[cfg(feature = "bindings")]
pub use self::bindings::*;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(not(feature = "no-entrypoint"))]
mod processor;

#[cfg(not(feature = "no-entrypoint"))]
pub use self::processor::*;

solana_program::declare_id!("octuswa5MD5hrTwcNBKvdxDvDQoz7C7M9sk2cRRvZfg");
