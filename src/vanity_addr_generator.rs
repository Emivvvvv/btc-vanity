//! # Vanity Address Generation Module
//!
//! This module is the core of btc-vanity. It provides the functionality to generate Bitcoin vanity addresses.
//!
//! # Example Usage
//!
//! ```rust
//! use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
//!
//! let vanity_address = VanityAddr::generate(
//!             "Test", // the string that you want your vanity address to include.
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

use crate::error::BtcVanityError;
use crate::keys_and_address::KeysAndAddress;

use bitcoin::secp256k1::{All, Secp256k1};
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use regex::Regex;

/// An Empty Struct for a more structured code
/// implements the only public function generate
pub struct VanityAddr;

/// Vanity mode enum
#[derive(Copy, Clone, Debug)]
pub enum VanityMode {
    Prefix,
    Suffix,
    Anywhere,
    Regex
}

impl VanityAddr {
    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns Ok if all checks were successful.
    /// Returns Err if the string is longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), BtcVanityError> {
        if string.is_empty() {
            return Ok(());
        }

        if string.len() > 4 && fast_mode {
            return Err(BtcVanityError::VanityGeneratorError(
                    "You're asking for too much!\n\
                    If you know this will take for a long time and really want to find something longer than 4 characters\n\
                    disable fast mode with -df or --disable_fast flags.",
                ));
        }

        let is_base58 = string
            .chars()
            .any(|c| c == '0' || c == 'I' || c == 'O' || c == 'l' || !c.is_alphanumeric());

        if is_base58 {
            return Err(BtcVanityError::VanityGeneratorError(
                    "Your input is not in base58. Don't include zero: '0', uppercase i: 'I', uppercase o: 'O', lowercase L: 'l' \
                    or any non-alphanumeric character in your input!",
                ));
        }

        Ok(())
    }

    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns Result<KeysAndAddressString, VanityGeneratorError>
    /// Returns OK if a vanity address found successfully with keys_and_address::KeysAndAddress struct
    /// Returns Err if the string is longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    pub fn generate(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<KeysAndAddress, BtcVanityError> {
        let secp256k1 = Secp256k1::new();

        Self::validate_input(string, fast_mode)?;

        if string.is_empty() {
            return Ok(KeysAndAddress::generate_random(&secp256k1));
        }

        Ok(SearchEngines::find_vanity_address(
            string,
            threads,
            case_sensitive,
            vanity_mode,
            secp256k1,
        ))
    }
}

/// impl's `find_vanity_address_fast_engine` and `find_vanity_address_fast_engine_with_range`
pub struct SearchEngines;

impl SearchEngines {
    /// Search for the vanity address with given threads.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements,
    /// it sends the `KeysAndAddress` via an `mpsc` channel. The main thread then signals
    /// all other threads to stop via an `AtomicBool`.
    fn find_vanity_address(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        secp256k1: Secp256k1<All>,
    ) -> KeysAndAddress {
        let string_len = string.len();
        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = found_any.clone();

            let secp256k1 = secp256k1.clone();
            let string = string.to_string();
            let lowered_string = string.to_lowercase();
            let mut anywhere_flag = false;
            let mut prefix_suffix_flag = false;

            thread::spawn(move || {
                // Each thread runs until 'found_any' is set to true
                while !found_any.load(Ordering::Relaxed) {
                    let keys_and_address = KeysAndAddress::generate_random(&secp256k1);
                    let address = keys_and_address.get_comp_address();

                    match vanity_mode {
                        VanityMode::Prefix => {
                            let slice = &address[1..=string_len];
                            prefix_suffix_flag = match case_sensitive {
                                true => slice == string,
                                false => slice.to_lowercase() == lowered_string,
                            };
                        }
                        VanityMode::Suffix => {
                            let address_len = address.len();
                            let slice = &address[address_len - string_len..address_len];
                            prefix_suffix_flag = match case_sensitive {
                                true => slice == string,
                                false => slice.to_lowercase() == lowered_string,
                            };
                        }
                        VanityMode::Anywhere => {
                            anywhere_flag = match case_sensitive {
                                true => address.contains(&string),
                                false => address.to_lowercase().contains(&lowered_string),
                            };
                        }
                        VanityMode::Regex => unreachable!()
                    }

                    // If match found...
                    if (prefix_suffix_flag || anywhere_flag) && !found_any.load(Ordering::Relaxed) {
                        // Mark as found (and check if we are the first)
                        if !found_any.swap(true, Ordering::Relaxed) {
                            // We're the first thread to set found_any = true
                            // Attempt to send the result
                            let _ = sender.send(keys_and_address);
                        }
                        // Return immediately: no need to generate more
                        return;
                    }
                }
            });
        }

        // The main thread just waits for the first successful result.
        // As soon as one thread sends over the channel, we have our vanity address.
        receiver.recv().expect("Receiver closed before a vanity address was found")
    }

    /// Search for the vanity address satisfies the given Regex.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements,
    /// it sends the `KeysAndAddress` via an `mpsc` channel. The main thread then signals
    /// all other threads to stop via an `AtomicBool`.
    pub fn find_vanity_address_regex(
        regex_str: &str,
        threads: u64,
        secp256k1: Secp256k1<All>,
    ) -> Result<KeysAndAddress, BtcVanityError> {
        // If the user gave a pattern that starts with '^' but not '^1',
        // insert '1' right after '^'.
        //
        // Example:
        // ^E.*T$  ==>  ^1E.*T$
        let mut pattern_str = regex_str.to_string();
        if pattern_str.starts_with('^') && !pattern_str.starts_with("^1") {
            pattern_str.insert_str(1, "1");
        }

        let pattern = Arc::new(Regex::new(&pattern_str).map_err(|e| BtcVanityError::InvalidRegex)?);

        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = Arc::clone(&found_any);

            let secp256k1 = secp256k1.clone();
            let pattern = Arc::clone(&pattern);

            thread::spawn(move || {
                while !found_any.load(Ordering::Relaxed) {
                    let keys_and_address = KeysAndAddress::generate_random(&secp256k1);
                    let address = keys_and_address.get_comp_address();

                    // Check if this address matches the given regex
                    if pattern.is_match(address) && !found_any.load(Ordering::Relaxed) {
                        // Mark as found (and check if we are the first)
                        if !found_any.swap(true, Ordering::Relaxed) {
                            // We're the first thread to set found_any = true
                            // Attempt to send the result
                            let _ = sender.send(keys_and_address);
                        }
                        // Stop this thread immediately
                        return;
                    }
                }
            });
        }

        // The main thread just waits for the first successful result.
        // As soon as one thread sends over the channel, we have our vanity address.
        Ok(receiver.recv().expect("Receiver closed before a matching address was found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_vanity_prefix() {
        let vanity_string = "et";
        let keys_and_address = VanityAddr::generate(
            vanity_string,
            4,                  // Use 4 threads
            true,               // Case-insensitivity
            true,               // Fast mode (limits string size with 4 characters)
            VanityMode::Prefix, // Vanity mode set to Prefix
        )
        .unwrap();

        let vanity_addr_starts_with = "1et";
        assert!(keys_and_address
            .get_comp_address()
            .starts_with(vanity_addr_starts_with));
    }

    #[test]
    fn test_generate_vanity_suffix() {
        let vanity_string = "12";
        let keys_and_address = VanityAddr::generate(
            vanity_string,
            4,                  // Use 4 threads
            false,              // Case-insensitivity
            true,               // Fast mode (limits string size with 4 characters)
            VanityMode::Suffix, // Vanity mode set to Suffix
        )
        .unwrap();

        assert!(keys_and_address.get_comp_address().ends_with(vanity_string));
    }

    #[test]
    fn test_generate_vanity_anywhere() {
        let vanity_string = "ab";
        let keys_and_address = VanityAddr::generate(
            vanity_string,
            4,                    // Use 4 threads
            true,                 // Case-insensitivity
            true,                 // Fast mode (limits string size with 4 characters)
            VanityMode::Anywhere, // Vanity mode set to Anywhere
        )
        .unwrap();

        assert!(keys_and_address.get_comp_address().contains(vanity_string));
    }

    #[test]
    #[should_panic(expected = "You're asking for too much!")]
    fn test_generate_vanity_string_too_long_with_fast_mode() {
        let vanity_string = "12345"; // String longer than 4 characters
        let _ = VanityAddr::generate(
            vanity_string,
            4,                  // Use 4 threads
            false,              // Case-insensitivity
            true,               // Fast mode (limits string size with 4 characters)
            VanityMode::Prefix, // Vanity mode set to Prefix
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "Your input is not in base58.")]
    fn test_generate_vanity_invalid_base58() {
        let vanity_string = "emiO"; // Contains invalid base58 character 'O'
        let _ = VanityAddr::generate(
            vanity_string,
            4,                  // Use 4 threads
            false,              // Case-insensitivity
            true,               // Fast mode (limits string size with 4 characters)
            VanityMode::Prefix, // Vanity mode set to Prefix
        )
        .unwrap();
    }
}