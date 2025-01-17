#![allow(rustdoc::invalid_html_tags)]

//! # btc-vanity
//! btc-vanity is a Rust library and CLI tool that allows you to generate Bitcoin addresses that
//! contain a specific pattern or substring, known as a "vanity address." Whether you need a prefix,
//! suffix, or a specific string anywhere in the address, btc-vanity provides a highly customizable
//! and multithreaded solution to find your desired address quickly and efficiently.
//!
//! ## Why Use btc-vanity?
//! Bitcoin vanity addresses can be used to create recognizable, memorable, or branded addresses.
//! This tool leverages Rust's performance and safety features to deliver a reliable and
//! fast solution for generating these custom addresses.
//!
//! # Example Usage in Your Code: Creating a vanity address
//! ```rust
//! use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
//!
//! // Generate a vanity address with the desired pattern
//! let vanity_address = VanityAddr::generate(
//!             "Test", // The string that you want your vanity address to include.
//!             16,     // The number of threads to use for faster processing.
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
//! Creating a keypair
//! ```rust
//! use btc_vanity::keys_and_address::KeysAndAddress;
//!
//! // reason of using heavy is not importing bitcoin::secp256k1::Secp256k1
//! // if you want to create secp256k1 once and use it for every generation
//! // use KeysAndAddress::generate_random(secp256k1); instead.
//! let random_address = KeysAndAddress::generate_random_heavy();
//!
//! println!("A randomly generated key pair and their address\n\
//!           private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 random_address.get_wif_private_key(),
//!                 random_address.get_comp_public_key(),
//!                 random_address.get_comp_address())
//! ```
//! # Using btc-vanity via CLI
//!
//! After installing the crate, you can generate vanity addresses directly from the command line.
//! ```bash
//! $ cargo install btc-vanity
//! ```
//!
//! Then you can use --help or -h flags to learn how to use it!
//! ```bash
//! $ btc-vanity --help
//! A bitcoin vanity address generator written with the Rust programming language.
//!
//! Usage: btc-vanity [OPTIONS] [string]
//!
//! Arguments:
//! [string]  String used to match addresses.
//!
//! Options:
//! -i, --input-file <input-file>    File with strings to match addresses with.
//! Important: Write every string in a separate line.
//! -f, --force-flags                Use this flag to override the flags in the input file
//! or use in file to override cli flags for only that string.
//! Note: Cli -f is stronger than input-file -f.
//! -o, --output-file <output-file>  Crates a file that contains found wallet/s.
//! -p, --prefix                     Finds a vanity address which has 'string' prefix. [default]
//! -s, --suffix                     Finds a vanity address which has 'string' suffix.
//! -a, --anywhere                   Finds a vanity address which includes 'string' at any part of the address.
//! -t, --threads <threads>          Number of threads to be used. [default: 16]
//! -c, --case-sensitive             Use case sensitive comparison to match addresses.
//! -d, --disable-fast               Disables fast mode to find a prefix more than 4 characters.
//! -h, --help                       Print help
//! -V, --version                    Print version
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
