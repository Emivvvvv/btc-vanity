//! # Vanity Address Generation Module]
//!
//! This module is used to generate Bitcoin vanity addresses with multithreading.
//!
//! # Example Usage
//!
//! ```rust
//! use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
//!
//! let vanity_address = VanityAddr::generate(
//!             "Test", // the string that you want your vanity address include.
//!             16, // number of threads
//!             false, // case sensitivity (false ex: tESt, true ex: Test)
//!             false, // fast mode flag (to use a string longer than 4 chars this must be set to true)
//!             VanityMode::Anywhere).unwrap(); // vanity mode flag (prefix, suffix, anywhere available)
//!
//! println!("private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 vanity_address.get_wif_private_key(),
//!                 vanity_address.get_comp_public_key(),
//!                 vanity_address.get_comp_address())
//! ```

use crate::keys_and_address::KeysAndAddress;
use crate::error::CustomError;
use std::thread;
use std::sync::mpsc;

/// An Empty Struct for a more structured code
pub struct VanityAddr;

/// Vanity mode enum
#[derive(Copy, Clone)]
pub enum  VanityMode {
    Prefix,
    Suffix,
    Anywhere,
}

impl VanityAddr {
    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns a Result type
    /// Returns OK if a vanity address found successfully with keys_and_address::KeysAndAddress struct
    /// Returns Err if the string if longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    pub fn generate(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode
    ) -> Result<KeysAndAddress, CustomError> {
        if string.is_empty() { return Ok( KeysAndAddress::generate_random()) }
        if string.len() > 4 && fast_mode {
            return Err(CustomError("You're asking for too much!\n\
            If you know this will take for a long time and really want to find something longer than 4 characters\n\
             disable fast mode with -df or --disable_fas flags."))
        }

        let is_base58 = string
            .chars()
            .find(|c| c == &'0' || c == &'I' || c == &'O' || c == &'l');

        if is_base58.is_some() {
            return Err(CustomError("Your input is not in base58. Don't include zero: '0', uppercase i: 'I', uppercase o: 'O', lowercase L: 'l', in your input!"))
        }

        Ok(find_vanity_address(string, threads, case_sensitive, vanity_mode))
    }
}

/// Search for the vanity address with given threads.
/// First come served! If a thread finds a vanity address that satisfy all the requirements it sends
/// the keys_and_address::KeysAndAddress struct wia std::sync::mpsc channel and find_vanity_address function kills all of the other
/// threads and closes the channel and returns the found KeysAndAddress struct that includes
/// key pair and the desired address.
fn find_vanity_address(string: &str, threads: u64, case_sensitive: bool, vanity_mode: VanityMode) -> KeysAndAddress {
    let string_len = string.len();
    let (sender, receiver) = mpsc::channel();

    for _ in 0..threads {
        let sender = sender.clone();
        let string = string.to_string();
        let mut anywhere_flag = false;
        let mut prefix_suffix_flag = false;

        let _ = thread::spawn(move || {
            loop {
                let new_pair = KeysAndAddress::generate_random();
                let address = new_pair.get_comp_address();

                match vanity_mode {
                    VanityMode::Prefix => {
                        let slice = &address[1..=string_len];
                        prefix_suffix_flag = match case_sensitive {
                            true => slice == string,
                            false => slice.to_lowercase() == string.to_lowercase(),
                        };
                    }
                    VanityMode::Suffix => {
                        let address_len = address.len();
                        let slice = &address[address_len - string_len..address_len];
                        prefix_suffix_flag = match case_sensitive {
                            true => slice == string,
                            false => slice.to_lowercase() == string.to_lowercase(),
                        };
                    }
                    VanityMode::Anywhere => {
                        anywhere_flag = match case_sensitive {
                            true => address.contains(&string),
                            false => address.to_lowercase().contains(&string.to_lowercase()),
                        };
                    }
                }
                if prefix_suffix_flag || anywhere_flag {if sender.send(new_pair).is_err() {return}} // If the channel closed, that means another thread found a keypair and closed it so we just return and kill the thread if an error occurs.
            }
        });
    }

    loop {
        match receiver.try_recv() {
            Ok(pair) => return pair,
            Err(_) => continue
        }
    }
}