pub mod client;
pub mod message;
pub mod error;
pub mod result;
pub mod ops;

#[cfg(not(any(target_arch = "wasm32", target_os = "solana")))]
pub mod server;