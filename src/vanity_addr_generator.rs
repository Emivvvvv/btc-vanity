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
use crate::keys_and_address::{AddressGenerator, BitcoinKeyPair};
use crate::utils::is_valid_base58_char;

use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

/// Allowed "meta" characters for a simple subset of regex usage.
const ALLOWED_REGEX_META: &[char] = &[
    '^', '$', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '-',
];

/// An Empty Struct for a more structured code
/// implements the only public function generate
pub struct VanityAddr;

/// Vanity mode enum
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VanityMode {
    Prefix,
    Suffix,
    Anywhere,
    Regex,
}

impl VanityAddr {
    /// Checks all given information before passing to the vanity address finder function.
    /// 1) If length > 4 and `fast_mode` is true, reject (too long).
    /// 2) If any character is not base58, reject.
    pub fn validate_input(string: &str, fast_mode: bool) -> Result<(), BtcVanityError> {
        // 1) If length > 4 and `fast_mode` is true, reject (too long).
        if string.len() > 4 && fast_mode {
            return Err(BtcVanityError::FastModeEnabled);
        }

        // 2) If any character is not base58, reject.
        if string.chars().any(|c| !is_valid_base58_char(c)) {
            return Err(BtcVanityError::InputNotBase58);
        }

        Ok(())
    }

    /// Checks regex input before passing to the vanity address finder function.
    /// 1) If it's a recognized regex meta character, allow it.
    /// 2) If it's alphanumeric, ensure it's valid base58
    /// 3) Neither a recognized meta char, nor a valid base58 alphanumeric => reject
    pub fn validate_regex_input(regex_str: &str) -> Result<(), BtcVanityError> {
        // For each character in the pattern:
        for c in regex_str.chars() {
            // 1) If it's a recognized regex meta character, allow it.
            if ALLOWED_REGEX_META.contains(&c) {
                continue;
            }

            // 2) If it's alphanumeric, ensure it's valid base58
            if c.is_alphanumeric() {
                if !is_valid_base58_char(c) {
                    return Err(BtcVanityError::RegexNotBase58);
                }
            } else {
                // 3) Neither a recognized meta char, nor a valid base58 alphanumeric => reject
                return Err(BtcVanityError::InvalidRegex);
            }
        }

        Ok(())
    }

    /// Checks all given information's before passing to the vanity address finder function.
    /// See `[validate_input]` for passing conditions.
    /// Returns Result<[keys_and_address::KeysAndAddress], VanityGeneratorError>
    pub fn generate(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<BitcoinKeyPair, BtcVanityError> {
        Self::validate_input(string, fast_mode)?;

        if string.is_empty() {
            return Ok(BitcoinKeyPair::generate_random());
        }

        Ok(SearchEngines::find_vanity_address(
            string,
            threads,
            case_sensitive,
            vanity_mode,
        ))
    }

    /// Checks regex before passing to the vanity address finder function.
    /// See [validate_regex_input] for passing conditions.
    /// Returns Result<[keys_and_address::KeysAndAddress], VanityGeneratorError>
    pub fn generate_regex(regex_str: &str, threads: u64) -> Result<BitcoinKeyPair, BtcVanityError> {
        Self::validate_regex_input(regex_str)?;

        if regex_str.is_empty() {
            return Ok(BitcoinKeyPair::generate_random());
        }

        SearchEngines::find_vanity_address_regex(regex_str, threads)
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
    ) -> BitcoinKeyPair {
        let string_len = string.len();
        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = found_any.clone();

            let string = string.to_string();
            let lowered_string = string.to_lowercase();
            let mut anywhere_flag = false;
            let mut prefix_suffix_flag = false;

            thread::spawn(move || {
                // Each thread runs until 'found_any' is set to true
                while !found_any.load(Ordering::Relaxed) {
                    let keys_and_address = BitcoinKeyPair::generate_random();
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
                        VanityMode::Regex => unreachable!(),
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
        receiver
            .recv()
            .expect("Receiver closed before a vanity address was found")
    }

    /// Search for the vanity address satisfies the given Regex.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements,
    /// it sends the `KeysAndAddress` via an `mpsc` channel. The main thread then signals
    /// all other threads to stop via an `AtomicBool`.
    pub fn find_vanity_address_regex(
        regex_str: &str,
        threads: u64,
    ) -> Result<BitcoinKeyPair, BtcVanityError> {
        // If the user gave a pattern that starts with '^' but not '^1',
        // insert '1' right after '^'.
        //
        // Example:
        // ^E.*T$  ==>  ^1E.*T$
        let mut pattern_str = regex_str.to_string();
        if pattern_str.starts_with('^') && !pattern_str.starts_with("^1") {
            pattern_str.insert(1, '1');
        }

        let pattern =
            Arc::new(Regex::new(&pattern_str).map_err(|_e| BtcVanityError::InvalidRegex)?);

        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = Arc::clone(&found_any);

            let pattern = Arc::clone(&pattern);

            thread::spawn(move || {
                while !found_any.load(Ordering::Relaxed) {
                    let keys_and_address = BitcoinKeyPair::generate_random();
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
        Ok(receiver
            .recv()
            .expect("Receiver closed before a matching address was found"))
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
    #[should_panic(expected = "FastModeEnabled")]
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
    #[should_panic(expected = "InputNotBase58")]
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

    #[test]
    fn test_generate_regex_et_ends() {
        let pattern = "ET$";
        let keys_and_address =
            VanityAddr::generate_regex(pattern, 4).expect("Failed to generate address for 'ET$'");
        let address = keys_and_address.get_comp_address();

        // The final pattern is "ET$" => ends with "ET"
        assert!(
            address.ends_with("ET"),
            "Address should end with 'ET': {}",
            address
        );
    }

    #[test]
    fn test_generate_regex_rewrite() {
        // Original pattern is '^E' (not '^1'), so the code will insert '1', resulting in '^1E'.
        // We expect it eventually to find an address starting with "1E".
        let pattern = "^E";
        let keys_and_address = VanityAddr::generate_regex(pattern, 4).unwrap();
        let address = keys_and_address.get_comp_address();
        // Now that we know it's '^1E', check the first two characters:
        assert!(
            address.starts_with("1E"),
            "Address should start with '1E': {}",
            address
        );
    }

    #[test]
    fn test_generate_regex_e_any_t() {
        // Must start with "1E" (rewritten from "^E") and end with "T".
        let pattern = "^E.*T$";
        let keys_and_address = VanityAddr::generate_regex(pattern, 4)
            .expect("Failed to generate address for '^E.*T$'");
        let address = keys_and_address.get_comp_address();

        // Because of rewriting, the actual pattern used is '^1E.*T$'.
        // 1) Check it starts with "1E"
        assert!(
            address.starts_with("1E"),
            "Address should start with '1E': {}",
            address
        );
        // 2) Check it ends with 'T'
        assert!(
            address.ends_with('T'),
            "Address should end with 'T': {}",
            address
        );
    }

    #[test]
    fn test_generate_regex_e_69_any_t() {
        // Must start with "1E", contain "69", and end with "T".
        // Rewritten from "^E.*69.*T$" => "^1E.*69.*T$"
        let pattern = "^E.*69.*T$";
        let keys_and_address = VanityAddr::generate_regex(pattern, 4)
            .expect("Failed to generate address for '^E.*69.*T$'");
        let address = keys_and_address.get_comp_address();

        // After rewriting: '^1E.*69.*T$'
        assert!(
            address.starts_with("1E"),
            "Address should start with '1E': {}",
            address
        );
        assert!(
            address.contains("69"),
            "Address should contain '69': {}",
            address
        );
        assert!(
            address.ends_with('T'),
            "Address should end with 'T': {}",
            address
        );
    }

    #[test]
    #[should_panic(expected = "InvalidRegex")]
    fn test_generate_regex_invalid_syntax() {
        let pattern = "^(abc";
        let _ = VanityAddr::generate_regex(pattern, 4).unwrap();
    }

    #[test]
    #[should_panic(expected = "RegexNotBase58")]
    fn test_generate_regex_forbidden_char_zero() {
        let pattern = "^0";
        let _ = VanityAddr::generate_regex(pattern, 4).unwrap();
    }

    #[test]
    #[should_panic(expected = "RegexNotBase58")]
    fn test_generate_regex_forbidden_char_o() {
        let pattern = "^O";
        let _ = VanityAddr::generate_regex(pattern, 4).unwrap();
    }

    #[test]
    #[should_panic(expected = "RegexNotBase58")]
    fn test_generate_regex_forbidden_char_i() {
        let pattern = "^I";
        let _ = VanityAddr::generate_regex(pattern, 4).unwrap();
    }
}
