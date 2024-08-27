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
//!
//! ```rust
//! use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
//! use num_bigint::BigUint;
//! use num_traits::Num;
//!
//! fn main() {
//!     // Define the minimum range for the private key in hexadecimal format.
//!     let range_min = BigUint::from_str_radix("0000000000000000000000000000000000000000000000100000000000000000", 16).unwrap();
//!
//!     // Define the maximum range for the private key in hexadecimal format.
//!     let range_max = BigUint::from_str_radix("00000000000000000000000000000000000000000000001FFFFFFFFFFFFFFFFF", 16).unwrap();
//!
//!     // Generate a vanity address with the desired pattern within the specified range.
//!     let vanity_address = VanityAddr::generate_within_range(
//!         "abc",          // The string that you want your vanity address to include (as a prefix in this case).
//!         range_min,      // The minimum value of the private key range.
//!         range_max,      // The maximum value of the private key range.
//!         16,             // The number of threads to use for faster processing.
//!         true,           // Case sensitivity flag: true means exact match, false means case-insensitive match.
//!         true,           // Fast mode flag: enables fast mode, limiting the string length to 4 characters.
//!         VanityMode::Prefix, // Where to match the string in the address (Prefix in this case).
//!     ).unwrap();         // Unwrap the result to get the generated address or panic on error.
//!
//!     // Print the generated private key, public key, and address in their respective formats.
//!     println!(
//!         "private_key (wif): {}\n\
//!         public_key (compressed): {}\n\
//!         address (compressed): {}\n\n",
//!         vanity_address.get_wif_private_key(),
//!         vanity_address.get_comp_public_key(),
//!         vanity_address.get_comp_address()
//!     )
//! }
//! ```

use crate::error::BtcVanityError;
use crate::keys_and_address::KeysAndAddress;
use bitcoin::secp256k1::{All, Secp256k1};
use num_bigint::BigUint;
use num_traits::Num;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

/// An Empty Struct for a more structured code
/// implements the only public function generate
pub struct VanityAddr;

/// Vanity mode enum
#[derive(Copy, Clone, Debug)]
pub enum VanityMode {
    Prefix,
    Suffix,
    Anywhere,
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

    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns Result<KeysAndAddressString, VanityGeneratorError>
    /// Returns OK if a vanity address found successfully with keys_and_address::KeysAndAddress struct
    /// Returns Err if the string is longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    /// Returns Err if something went wrong while generating keypair within range
    pub fn generate_within_range(
        string: &str,
        range_min: BigUint,
        range_max: BigUint,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<KeysAndAddress, BtcVanityError> {
        let secp256k1 = Secp256k1::new();

        Self::validate_input(string, fast_mode)?;

        if string.is_empty() {
            return KeysAndAddress::generate_within_range(&secp256k1, &range_min, &range_max, true);
        }

        SearchEngines::find_vanity_address_within_range(
            string,
            range_min,
            range_max,
            threads,
            case_sensitive,
            vanity_mode,
            secp256k1,
        )
    }
}

/// impl's `find_vanity_address_fast_engine` and `find_vanity_address_fast_engine_with_range`
pub struct SearchEngines;

impl SearchEngines {
    /// Search for the vanity address with given threads.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements it sends
    /// the keys_and_address::KeysAndAddress struct wia std::sync::mpsc channel and find_vanity_address function kills all the other
    /// threads and closes the channel and returns the found KeysAndAddress struct that includes
    /// key pair and the desired address.
    fn find_vanity_address(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        secp256k1: Secp256k1<All>,
    ) -> KeysAndAddress {
        let string_len = string.len();
        let (sender, receiver) = mpsc::channel();

        for _ in 0..threads {
            let sender = sender.clone();
            let string = string.to_string();
            let mut anywhere_flag = false;
            let mut prefix_suffix_flag = false;
            let secp256k1 = secp256k1.clone();

            let _ = thread::spawn(move || {
                loop {
                    let keys_and_address = KeysAndAddress::generate_random(&secp256k1);
                    let address = keys_and_address.get_comp_address();

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
                    // If the channel closed, that means another thread found a keypair and closed it
                    // so we just return and kill the thread if an error occurs.
                    if (prefix_suffix_flag || anywhere_flag)
                        && sender.send(keys_and_address).is_err()
                    {
                        return;
                    }
                }
            });
        }

        loop {
            match receiver.try_recv() {
                Ok(pair) => return pair,
                Err(_) => continue,
            }
        }
    }

    /// Search for the vanity address with given threads within given range.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements it sends
    /// the keys_and_address::KeysAndAddress struct wia std::sync::mpsc channel and find_vanity_address function kills all the other
    /// threads and closes the channel and returns the found KeysAndAddress struct that includes
    /// key pair and the desired address.
    /// returns error if there is no match withing given range.
    fn find_vanity_address_within_range(
        string: &str,
        range_min: BigUint,
        range_max: BigUint,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        secp256k1: Secp256k1<All>,
    ) -> Result<KeysAndAddress, BtcVanityError> {
        let string_len = string.len();
        let (sender, receiver) = mpsc::channel();

        // Ensure range_max is greater than range_min
        if range_max <= range_min {
            return Err(BtcVanityError::VanityGeneratorError(
                "range_max must be greater than range_min",
            ));
        }

        if range_min == BigUint::ZERO {
            return Err(BtcVanityError::VanityGeneratorError("range_min can't be 0"));
        }

        // Private key range_max must be within the valid range for Secp256k1
        let secp256k1_order = BigUint::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .map_err(|_| BtcVanityError::VanityGeneratorError("Failed to parse hexadecimal string"))?;

        if range_max > secp256k1_order {
            return Err(BtcVanityError::VanityGeneratorError(
                "range_max must be within the valid range for Secp256k1",
            ));
        }

        // Calculate the total range size
        let range_size = &range_max - &range_min;

        // Divide the range equally among the threads
        let step = &range_size / BigUint::from(threads);

        // Track the number of threads that have finished their work
        let finished_threads = Arc::new(AtomicUsize::new(0));

        for i in 0..threads {
            let sender = sender.clone();
            let string = string.to_string();
            let mut anywhere_flag = false;
            let mut prefix_suffix_flag = false;
            let secp256k1 = secp256k1.clone();
            let finished_threads = Arc::clone(&finished_threads);

            // Calculate the starting point for this thread
            let thread_range_min = &range_min + (&step * BigUint::from(i));
            let thread_range_max = if i == threads - 1 {
                range_max.clone()
            } else {
                &thread_range_min + &step
            };

            let _ = thread::spawn(move || {
                let mut key_value = thread_range_min.clone();

                loop {
                    // If the current key value exceeds the max range for this thread, exit the loop
                    if key_value > thread_range_max {
                        break;
                    }

                    // Generate the key pair and address using generate_from_biguint
                    let keys_and_address = match KeysAndAddress::generate_from_biguint(
                        &secp256k1, &key_value, false,
                    ) {
                        Ok(keys) => keys,
                        Err(_) => return,
                    };

                    let address = keys_and_address.get_comp_address();

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
                    // If the channel is closed, another thread found a keypair, so we kill this thread
                    if (prefix_suffix_flag || anywhere_flag)
                        && sender.send(keys_and_address).is_err()
                    {
                        return;
                    }

                    // Increment the key value and continue
                    key_value += BigUint::from(1u64);
                }

                // Indicate that this thread has finished
                finished_threads.fetch_add(1, Ordering::SeqCst);
            });
        }

        loop {
            match receiver.try_recv() {
                Ok(pair) => return Ok(pair),
                Err(mpsc::TryRecvError::Empty) => {
                    // Check if all threads have finished their work
                    if finished_threads.load(Ordering::SeqCst) == threads as usize {
                        return Err(BtcVanityError::VanityGeneratorError(
                            "Vanity address not found within the given range",
                        ));
                    }
                }
                Err(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;
    use num_traits::{FromPrimitive, Num};

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

    #[test]
    fn test_generate_within_range_prefix() {
        let vanity_string = "em";
        let range_min = BigUint::from_u64(1).unwrap();
        let range_max = BigUint::from_u64(u64::MAX).unwrap();

        #[allow(deprecated)]
        let keys_and_address = VanityAddr::generate_within_range(
            vanity_string,
            range_min,
            range_max,
            4,                  // Use 4 threads
            true,               // Case-insensitivity
            true,               // Fast mode (limits string size with 4 characters)
            VanityMode::Prefix, // Vanity mode set to Prefix
        )
        .unwrap();

        let vanity_addr_starts_with = "1em";
        assert!(keys_and_address
            .get_comp_address()
            .starts_with(vanity_addr_starts_with));
    }

    #[test]
    fn test_generate_within_range_anywhere() {
        let vanity_string = "ab";
        let range_min = BigUint::from_u64(1).unwrap();
        let range_max = BigUint::from_u64(u64::MAX).unwrap();

        #[allow(deprecated)]
        let keys_and_address = VanityAddr::generate_within_range(
            vanity_string,
            range_min,
            range_max,
            4,                    // Use 4 threads
            true,                 // Case-insensitivity
            true,                 // Fast mode (limits string size with 4 characters)
            VanityMode::Anywhere, // Vanity mode set to Anywhere
        )
        .unwrap();

        assert!(keys_and_address.get_comp_address().contains(vanity_string));
    }

    #[test]
    #[should_panic(expected = "Vanity address not found within the given range")]
    fn test_generate_within_range_loop_proof_not_found() {
        let vanity_string = "abc";
        let range_min = BigUint::from_str_radix(
            "0000000000000000000000000000000000000000000000000000000000100000",
            16,
        )
        .unwrap();
        let range_max = BigUint::from_str_radix(
            "00000000000000000000000000000000000000000000000000000000001FFFFF",
            16,
        )
        .unwrap();

        let _ = VanityAddr::generate_within_range(
            vanity_string,
            range_min,
            range_max,
            16,
            true,
            true,
            VanityMode::Prefix,
        )
        .unwrap();
    }
}
