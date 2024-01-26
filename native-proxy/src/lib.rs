mod instruction;

pub use self::instruction::*;

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

solana_program::declare_id!("WrapR8ncp6aGqux2TACyJh4MUxcHAHTW9eYzzeXuTJA");
