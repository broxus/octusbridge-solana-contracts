mod instruction;
mod processor;
mod state;
mod utils;

pub use self::instruction::*;
pub use self::processor::*;
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

solana_program::declare_id!("roundAsiEM445bGEp7ZwPWXUmWAHh6rpLEndJUKP1V4");
