//! # btc-vanity
//!
//! `btc-vanity` searches locally generated Bitcoin, Ethereum, and Solana
//! keypairs for addresses that match a requested pattern. It supports prefix,
//! suffix, substring, and regular-expression matching through its command-line
//! application and Rust API.
//!
//! CPU search is always available for Bitcoin. Cargo features add Ethereum,
//! Solana, and the experimental GPU backend. GPU results are reconstructed on
//! the CPU before they are returned.
//!
//! ## Example
//!
//! ```
//! use btc_vanity::{
//!     BitcoinKeyPair, VanityAddr, VanityBackend, VanityMode, VanitySearchOptions,
//! };
//!
//! let wallet = VanityAddr::generate_with_options::<BitcoinKeyPair>(
//!     "cat",
//!     VanitySearchOptions {
//!         threads: 4,
//!         case_sensitive: false,
//!         vanity_mode: VanityMode::Prefix,
//!         backend: VanityBackend::Cpu,
//!         ..VanitySearchOptions::default()
//!     },
//! )?;
//!
//! println!("address: {}", wallet.get_comp_address());
//! # Ok::<(), btc_vanity::error::VanityError>(())
//! ```

pub const BATCH_SIZE: usize = 256;

pub mod cli;
pub mod error;
pub mod file;
pub mod flags;
pub mod keys_and_address;
pub mod vanity_addr_generator;
#[cfg(feature = "gpu")]
pub(crate) mod wgpu_sig_ops;

#[cfg(feature = "ethereum")]
pub use crate::keys_and_address::EthereumKeyPair;
#[cfg(feature = "solana")]
pub use crate::keys_and_address::SolanaKeyPair;
pub use crate::keys_and_address::{BitcoinKeyPair, KeyPairGenerator};
pub use vanity_addr_generator::vanity_addr::{
    GpuCurveKind, VanityAddr, VanityBackend, VanityMode, VanitySearchOptions,
};
