#![allow(rustdoc::invalid_html_tags)]
//! # btc-vanity
//!
//! `btc-vanity` is a blazingly fast Rust library and CLI tool designed for generating **vanity cryptocurrency addresses**.
//! Whether you are looking to create Bitcoin, Ethereum, or Solana addresses with specific patterns or substrings,
//! `btc-vanity` offers a customizable and highly performant solution to achieve your goals.
//!
//! With support for **prefix**, **suffix**, **substring**, or even **regex-based patterns**, this library
//! ensures you can generate your desired address with ease. Built with multithreaded support, it maximizes
//! performance to quickly find vanity addresses for various blockchains.
//!
//! ### Example Usages
//!
//! You can easily generate a random Bitcoin keypair and print the private and public keys along with the address:
//!
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, KeyPairGenerator};
//!
//! let random_address = BitcoinKeyPair::generate_random();
//!
//! println!("Randomly generated Bitcoin key pair:\n\
//!           private_key (WIF): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n",
//!           random_address.get_wif_private_key(),
//!           random_address.get_comp_public_key(),
//!           random_address.get_comp_address());
//! ```
//!
//! Find a Bitcoin address that contains the substring `emiv` (case-insensitive) using 16 threads:
//!
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, VanityAddr, VanityMode};
//!
//! let vanity_address: BitcoinKeyPair = VanityAddr::generate(
//! "emiv", // Desired substring
//! 16,     // Number of threads
//! false,  // Case-insensitive
//! true,   // Enable fast mode
//! VanityMode::Anywhere // Match substring anywhere in the address
//! ).unwrap();
//!
//! println!("Vanity address:\n\
//!           private_key (WIF): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n",
//!          vanity_address.get_wif_private_key(),
//!          vanity_address.get_comp_public_key(),
//!          vanity_address.get_comp_address());
//! ```
//!
//! Create a Bitcoin address with `meow` anywhere in the address (case-sensitive) using 4 threads:
//!
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, VanityAddr, VanityMode};
//!
//! let vanity_address: BitcoinKeyPair = VanityAddr::generate(
//! "meow",  // Desired substring
//! 4,      // Number of threads
//! true,  // Case-sensitive
//! true,   // Enable fast mode
//! VanityMode::Anywhere // Match substring anywhere in the address
//! ).unwrap();
//!
//! println!("Vanity address:\n\
//!           private_key (WIF): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n",
//!          vanity_address.get_wif_private_key(),
//!          vanity_address.get_comp_public_key(),
//!          vanity_address.get_comp_address());
//! ```
//!
//! Find a Bitcoin address that matches a regex pattern `^1E.ET.*T$` with using 12 threads:
//!
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, VanityAddr};
//!
//! let vanity_address = VanityAddr::generate_regex::<BitcoinKeyPair>(
//! "^1E.*ET.*T$", // The regex pattern
//! 12            // Number of threads
//! ).unwrap();
//!
//! println!("Bitcoin regex-matched vanity address:\n\
//!           private_key (WIF): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n",
//!          vanity_address.get_wif_private_key(),
//!          vanity_address.get_comp_public_key(),
//!          vanity_address.get_comp_address());
//! ```

pub const BATCH_SIZE: usize = 64;

pub mod cli;
pub mod error;
pub mod file;
pub mod flags;
pub mod keys_and_address;
pub mod vanity_addr_generator;

#[cfg(feature = "ethereum")]
pub use crate::keys_and_address::EthereumKeyPair;
#[cfg(feature = "solana")]
pub use crate::keys_and_address::SolanaKeyPair;
pub use crate::keys_and_address::{BitcoinKeyPair, KeyPairGenerator};
pub use vanity_addr_generator::vanity_addr::{VanityAddr, VanityMode};
