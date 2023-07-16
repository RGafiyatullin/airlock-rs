//! Airlock: no_std async communication

#![no_std]
#![warn(missing_debug_implementations, missing_docs)]

/// Atomic Waker
pub mod atomic_waker;
/// Errors.
pub mod error;
/// Multiple producers multiple consumers buffered channel.
pub mod mpmc;
/// Wrapper around unsafe-cell carrying a value.
pub mod slot;
/// Single producer single consumer channels
pub mod spsc;

mod fmt;
mod utils;

#[cfg(feature = "std")]
extern crate std;
