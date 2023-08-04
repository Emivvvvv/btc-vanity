//! # A rusty way to find your bitcoin vanity address!
//!
//! With btc-vanity you can create a private key which has a compressed bitcoin pay address
//! that has a custom prefix, suffix or a string at somewhere in the address.
//!
//! # Example Usage At Your Code
//! ```rust
//! use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
//!
//! let vanity_address = VanityAddr::generate(
//!             "Test", // the string that you want your vanity address include.
//!             16, // number of threads
//!             false, // case sensitivity (false ex: tESt, true ex: Test)
//!             true, // fast mode flag (to use a string longer than 4 chars this must be set to false)
//!             VanityMode::Anywhere, // vanity mode flag (prefix, suffix, anywhere available)
//!             ).unwrap(); // this function returns a result type
//!
//! println!("private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 vanity_address.get_wif_private_key(),
//!                 vanity_address.get_comp_public_key(),
//!                 vanity_address.get_comp_address())
//! ```
//!
//! # To Use At Your Cli
//!
//! First install the crate with
//!
//! ```bash
//! $ cargo install btc-vanity
//! ```
//!
//! Then you can use --help or -h flags to learn how to use it!
//!
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
//! Find a vanity address with prefix "Emiv" and appends the wallet details to -wallet.txt
//! (if there is no wallet.txt it crates a new one)
//! ```bash
//! $ btc-vanity -o wallet.txt Emiv
//! ```
//!
//! Gets all the inputs and flags (if available) from the inputs.txt text file
//! sets the vanity mode anywhere for the strings which don't have any vanity mode flag
//! and appends all the wallet details to -wallets.txt with using 8 threads
//! (if there is no wallets.txt it crates a new one)
//! ```bash
//! $ btc-vanity -i inputs.txt -o wallets.txt -t 8 -a
//! ```
//!
//! Gets all the inputs and flags (if available) from the inputs.txt text file
//! overrides all flags with the vanity mode to suffix, if a strings has it's own
//! -o <text file> flag it ignores it because of -f flag adn prints all the wallet details
//! to stdout.
//! ```bash
//! $ btc-vanity -f -s -i inputs.txt
//! ```


pub mod vanity_addr_generator;
pub mod keys_and_address;
pub mod error;
pub mod cli;
pub mod file;
pub mod decoration;
pub mod flags;