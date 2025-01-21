#![allow(rustdoc::invalid_html_tags)]

//! # btc-vanity
//! btc-vanity is a Rust library and CLI tool that allows you to generate Bitcoin addresses that
//! contain a specific pattern or substring, known as a "vanity address." Whether you need a prefix,
//! suffix, a specific string anywhere in the address, or even a pattern defined by a regular
//! expression, btc-vanity provides a highly customizable and multithreaded solution to find your
//! desired address quickly and efficiently.
//!
//! # Example Usage in Your Code
//!
//! Creating a keypair
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, KeyPairGenerator};
//!
//! let random_address = BitcoinKeyPair::generate_random();
//!
//! println!("A randomly generated key pair and their address\n\
//!           private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 random_address.get_wif_private_key(),
//!                 random_address.get_comp_public_key(),
//!                 random_address.get_comp_address())
//! ```
//!
//! Generating a vanity address that has a substring `test` (not case-sensitive) in our vanity
//! address with using 16 threads.
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, VanityAddr, VanityMode};
//!
//! // Generate a vanity address with the desired pattern
//! let vanity_address: BitcoinKeyPair = VanityAddr::generate(
//!             "Test", // The string that you want your vanity address to include.
//!             16,     // The number of threads
//!             false,  // Case sensitivity flag: false means "tESt" is valid, true means only "Test".
//!             true,   // Fast mode flag: enables fast mode, limiting the string length to 4 characters.
//!             VanityMode::Anywhere, // Where to match the string in the address (Prefix, Suffix, Anywhere).
//!             ).unwrap(); // Unwrap the result to get the generated address or panic on error.
//!
//! println!("private_key (WIF): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 vanity_address.get_wif_private_key(),
//!                 vanity_address.get_comp_public_key(),
//!                 vanity_address.get_comp_address())
//! ```
//!
//! Generating a vanity address that satisfies regex `^E.*99.*T$` (case-sensitive) in our vanity
//! address with using 8 threads.
//! ```rust
//! use btc_vanity::{BitcoinKeyPair, VanityAddr, VanityMode};
//!
//! // Generate a vanity address that satisfies given regex
//! let vanity_address = VanityAddr::generate_regex::<BitcoinKeyPair>(
//!             "^E.*99.*T$", // The regex as string
//!             16,     // The number of threads
//!             ).unwrap(); // Unwrap the result to get the generated address or panic on error.
//!
//! println!("private_key (WIF): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 vanity_address.get_wif_private_key(),
//!                 vanity_address.get_comp_public_key(),
//!                 vanity_address.get_comp_address())
//! ```
//!
//! # Using btc-vanity via CLI
//!
//! After installing the crate, you can generate vanity addresses directly from the command line.
//! ```bash
//! $ cargo install btc-vanity
//! $ btc-vanity -h
//! ```
//!
//! # Some Cli Usage Examples
//!
//! Finds a vanity address with prefix "Emiv" and appends the wallet details to -wallet.txt
//! (if there is no wallet.txt it crates a new one)
//! ```bash
//! $ btc-vanity -o wallet.txt Emiv
//! ```
//!
//! Gets all the substrings and flags (if available) from the inputs.txt text file
//! sets the vanity mode anywhere for the strings which don't have any vanity mode flag
//! runs on 8 threads.
//! Appends found wallet details to -wallets.txt (if there is no wallets.txt it crates a new one)
//! ```bash
//! $ btc-vanity -i inputs.txt -o wallets.txt -t 8 -a
//! ```
//!
//! Gets all the substrings from the `inputs.txt` text file. Because we set -f (force flag),
//! overrides all flags as -s (suffix). Even though a strings has its own -o <text file> flag,
//! they also will be ignored. Prints all the wallet details, right after finding one to stdout.
//! ```bash
//! $ btc-vanity -f -s -i inputs.txt
//! ```

pub mod cli;
pub mod decoration;
pub mod error;
pub mod file;
pub mod flags;
pub mod keys_and_address;
pub mod utils;
pub mod vanity_addr_generator;

pub use keys_and_address::{BitcoinKeyPair, EthereumKeyPair, KeyPairGenerator};
pub use vanity_addr_generator::vanity_addr::{VanityAddr, VanityMode};
