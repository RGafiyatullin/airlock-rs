//! Airlock: no_std async communication

#![no_std]
#![warn(missing_debug_implementations, missing_docs)]

/// Single producer single consumer buffered channel.
pub mod buffered;
/// Errors.
pub mod error;
/// Single producer single consumer non-buffered channel.
pub mod mono;
/// Wrapper around unsafe-cell carrying a value.
pub mod slot;

mod fmt;
mod utils;

#[cfg(feature = "std")]
extern crate std;
