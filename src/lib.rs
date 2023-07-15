#![no_std]

pub mod error;
pub mod mono;

mod fmt;
mod slot;

#[cfg(feature = "std")]
extern crate std;
