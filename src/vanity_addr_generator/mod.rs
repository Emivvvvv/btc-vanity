//! # Vanity Address Generation Module
//!
//! This module is the core of btc-vanity. It provides the functionality to generate Bitcoin vanity addresses.

pub mod chain;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod vanity_addr;

mod comp;
