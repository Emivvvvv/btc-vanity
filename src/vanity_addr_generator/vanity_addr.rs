use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::mem::MaybeUninit;

use crate::error::VanityError;
use crate::vanity_addr_generator::chain::Chain;
use crate::vanity_addr_generator::compx::{contains_memx, eq_prefix_memx, eq_suffix_memx,
};

use regex::Regex;

const BATCH_SIZE: usize = 100;

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
    /// Checks all given information's before passing to the vanity address finder function.
    /// See `[validate_input]` for passing conditions.
    /// Returns Result<[keys_and_address::KeysAndAddress], VanityGeneratorError>
    pub fn generate<T: Chain + 'static>(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<T, VanityError> {
        T::validate_input(string, fast_mode)?;
        let adjusted_string = T::adjust_input(string, vanity_mode);

        if string.is_empty() {
            return Ok(T::generate_random());
        }

        Ok(SearchEngines::find_vanity_address::<T>(
            adjusted_string,
            threads,
            case_sensitive,
            vanity_mode,
        ))
    }

    /// Checks regex before passing to the vanity address finder function.
    /// See [validate_regex_input] for passing conditions.
    /// Returns Result<[keys_and_address::KeysAndAddress], VanityGeneratorError>
    pub fn generate_regex<T: Chain + 'static>(
        regex_str: &str,
        threads: u64,
    ) -> Result<T, VanityError> {
        T::validate_regex_pattern(regex_str)?;
        let adjusted_regex = T::adjust_regex_pattern(regex_str);

        if regex_str.is_empty() {
            return Ok(T::generate_random());
        }

        SearchEngines::find_vanity_address_regex::<T>(adjusted_regex, threads)
    }
}

/// impl's `find_vanity_address_fast_engine` and `find_vanity_address_fast_engine_with_range`
pub struct SearchEngines;

fn generate_batch_of_pairs<T: Chain>() -> [T; BATCH_SIZE] {
    // Create an uninitialized array
    let mut batch: [MaybeUninit<T>; BATCH_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

    // Initialize each element in the array
    for item in batch.iter_mut() {
        *item = MaybeUninit::new(T::generate_random());
    }

    // SAFELY convert the initialized `MaybeUninit` array into a properly initialized array
    unsafe { std::mem::transmute_copy(&batch) }
}

impl SearchEngines {
    /// Search for the vanity address with given threads.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements,
    /// it sends the `KeysAndAddress` via an `mpsc` channel. The main thread then signals
    /// all other threads to stop via an `AtomicBool`.
    fn find_vanity_address<T: Chain + 'static>(
        string: String,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> T {
        let string_bytes = string.as_bytes();
        let string_len = string_bytes.len();
        let _string_len = string_bytes.len();
        let lower_string_bytes = if !case_sensitive {
            string_bytes
                .iter()
                .map(|b| b.to_ascii_lowercase())
                .collect::<Vec<u8>>()
        } else {
            vec![]
        };

        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = found_any.clone();

            let thread_string_bytes = string_bytes.to_vec();
            let thread_lower_string_bytes = lower_string_bytes.clone();

            thread::spawn(move || {
                while !found_any.load(Ordering::Relaxed) {
                    // Generate a batch of addresses
                    let batch = generate_batch_of_pairs::<T>();

                    // Check each address in the batch
                    for keys_and_address in batch {
                        let address_bytes = keys_and_address.get_address_bytes();

                        let matches = match vanity_mode {
                            VanityMode::Prefix => {
                                if case_sensitive {
                                    eq_prefix_memx(address_bytes, &thread_string_bytes)
                                } else {
                                    address_bytes[..string_len]
                                        .iter()
                                        .map(|b| b.to_ascii_lowercase())
                                        .collect::<Vec<u8>>()
                                        == thread_lower_string_bytes
                                }
                            }
                            VanityMode::Suffix => {
                                if case_sensitive {
                                    eq_suffix_memx(address_bytes, &thread_string_bytes)
                                } else {
                                    address_bytes[address_bytes.len() - string_len..]
                                        .iter()
                                        .map(|b| b.to_ascii_lowercase())
                                        .collect::<Vec<u8>>()
                                        == thread_lower_string_bytes
                                }
                            }
                            VanityMode::Anywhere => {
                                if case_sensitive {
                                    contains_memx(address_bytes, &thread_string_bytes)
                                } else {
                                    address_bytes.windows(string_len).any(|window| {
                                        window
                                            .iter()
                                            .map(|b| b.to_ascii_lowercase())
                                            .collect::<Vec<u8>>()
                                            == thread_lower_string_bytes
                                    })
                                }
                            }
                            VanityMode::Regex => unreachable!(),
                        };

                        // If match found...
                        if matches && !found_any.load(Ordering::Relaxed) {
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
    pub fn find_vanity_address_regex<T: Chain + 'static>(
        regex_str: String,
        threads: u64,
    ) -> Result<T, VanityError> {
        // Validate the regex syntax
        Arc::new(Regex::new(&regex_str).map_err(|_e| VanityError::InvalidRegex)?);

        thread_local! {
            static THREAD_REGEX: std::cell::RefCell<Option<Regex>> = const {std::cell::RefCell::new(None)};
        }

        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = Arc::clone(&found_any);
            let regex_clone = regex_str.clone();

            thread::spawn(move || {
                // Initialize the regex in thread-local storage.
                THREAD_REGEX.with(|local_regex| {
                    let mut local_regex_ref = local_regex.borrow_mut();
                    if local_regex_ref.is_none() {
                        *local_regex_ref = Some(Regex::new(&regex_clone).unwrap());
                    }

                    let regex = local_regex_ref.as_ref().unwrap();

                    while !found_any.load(Ordering::Relaxed) {
                        // Generate a batch of addresses
                        let batch = generate_batch_of_pairs::<T>();

                        // Check each address in the batch
                        for keys_and_address in batch {
                            let address = keys_and_address.get_address();
                            if regex.is_match(address) && !found_any.load(Ordering::Relaxed) {
                                // If a match is found, send it to the main thread
                                if !found_any.swap(true, Ordering::Relaxed) {
                                    sender.send(keys_and_address).unwrap();
                                    return;
                                }
                            }
                        }
                    }
                });
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
    use super::{VanityAddr, VanityMode};

    mod bitcoin_vanity_tests {
        use super::*;
        use crate::keys_and_address::BitcoinKeyPair;

        #[test]
        fn test_generate_vanity_prefix() {
            let vanity_string = "et";
            let keys_and_address = VanityAddr::generate::<BitcoinKeyPair>(
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
            let keys_and_address = VanityAddr::generate::<BitcoinKeyPair>(
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
            let keys_and_address = VanityAddr::generate::<BitcoinKeyPair>(
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
            let _ = VanityAddr::generate::<BitcoinKeyPair>(
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
            let _ = VanityAddr::generate::<BitcoinKeyPair>(
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
            let keys_and_address = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4)
                .expect("Failed to generate address for 'ET$'");
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
            let keys_and_address =
                VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
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
            let keys_and_address = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4)
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
            let keys_and_address = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4)
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
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_forbidden_char_zero() {
            let pattern = "^0";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_forbidden_char_o() {
            let pattern = "^O";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_forbidden_char_i() {
            let pattern = "^I";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }
    }

    mod ethereum_vanity_tests {
        use super::*;
        use crate::keys_and_address::{EthereumKeyPair, KeyPairGenerator};

        #[test]
        fn test_generate_vanity_prefix() {
            let vanity_string = "ab";
            let keys_and_address = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                true,               // Case-insensitivity
                true,               // Fast mode (limits string size to 12 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();

            let expected_prefix = "ab";
            assert!(keys_and_address
                .get_address()
                .to_lowercase()
                .starts_with(expected_prefix));
        }

        #[test]
        fn test_generate_vanity_suffix() {
            let vanity_string = "123";
            let keys_and_address = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode (limits string size to 12 characters)
                VanityMode::Suffix, // Vanity mode set to Suffix
            )
            .unwrap();

            assert!(keys_and_address.get_address().ends_with(vanity_string));
        }

        #[test]
        fn test_generate_vanity_anywhere() {
            let vanity_string = "abc";
            let keys_and_address = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                    // Use 4 threads
                true,                 // Case-insensitivity
                true,                 // Fast mode (limits string size to 12 characters)
                VanityMode::Anywhere, // Vanity mode set to Anywhere
            )
            .unwrap();

            assert!(keys_and_address.get_address().contains(vanity_string));
        }

        #[test]
        #[should_panic(expected = "FastModeEnabled")]
        fn test_generate_vanity_string_too_long_with_fast_mode() {
            let vanity_string = "1234567890123"; // String longer than 12 characters
            let _ = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode (limits string size to 12 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase16")]
        fn test_generate_vanity_invalid_base16() {
            let vanity_string = "g123"; // Contains invalid base16 character 'g'
            let _ = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode (limits string size to 12 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase16")]
        fn test_generate_vanity_with_prefix() {
            let vanity_string = "0xdead"; // Contains invalid base16 character 'x'
            let _ = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode (limits string size to 12 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        fn test_generate_regex_prefix() {
            let pattern = "^ab";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("ab"),
                "Address should start with 'ab': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_suffix() {
            let pattern = "cd$";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("cd"),
                "Address should end with 'cd': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_anywhere() {
            let pattern = ".*abc.*";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("abc"),
                "Address should contain 'abc': {}",
                address
            );
        }

        #[test]
        #[should_panic(expected = "InvalidRegex")]
        fn test_generate_regex_invalid_syntax() {
            let pattern = "^(abc";
            let _ = VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase16")]
        fn test_generate_regex_invalid_characters() {
            let pattern = "^gh";
            let _ = VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase16")]
        fn test_generate_regex_with_prefix() {
            let pattern = "^0xdead";
            let _ = VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        fn test_generate_regex_complex_pattern() {
            let pattern = "^ab.*12$";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("ab"),
                "Address should start with 'ab': {}",
                address
            );
            assert!(
                address.ends_with("12"),
                "Address should end with '12': {}",
                address
            );
        }
    }

    mod solana_vanity_tests {
        use super::*;
        use crate::keys_and_address::{KeyPairGenerator, SolanaKeyPair};

        #[test]
        fn test_generate_vanity_prefix() {
            let vanity_string = "et";
            let keys_and_address = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                true,               // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();

            let vanity_addr_starts_with = "et";
            assert!(keys_and_address
                .get_address()
                .starts_with(vanity_addr_starts_with));
        }

        #[test]
        fn test_generate_vanity_suffix() {
            let vanity_string = "12";
            let keys_and_address = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Suffix, // Vanity mode set to Suffix
            )
            .unwrap();

            assert!(keys_and_address.get_address().ends_with(vanity_string));
        }

        #[test]
        fn test_generate_vanity_anywhere() {
            let vanity_string = "ab";
            let keys_and_address = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                    // Use 4 threads
                true,                 // Case-insensitivity
                true,                 // Fast mode (limits string size with 44 characters)
                VanityMode::Anywhere, // Vanity mode set to Anywhere
            )
            .unwrap();

            assert!(keys_and_address.get_address().contains(vanity_string));
        }

        #[test]
        #[should_panic(expected = "FastModeEnabled")]
        fn test_generate_vanity_string_too_long_with_fast_mode() {
            let vanity_string = "12345"; // String longer than 4 characters
            let _ = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase58")]
        fn test_generate_vanity_invalid_base58() {
            let vanity_string = "emiO"; // Contains invalid base58 character 'O'
            let _ = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        fn test_generate_regex_prefix() {
            let pattern = "^et";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("et"),
                "Address should start with 'et': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_suffix() {
            let pattern = "cd$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("cd"),
                "Address should end with 'cd': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_anywhere() {
            let pattern = ".*ab.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("ab"),
                "Address should contain 'ab': {}",
                address
            );
        }

        #[test]
        #[should_panic(expected = "InvalidRegex")]
        fn test_generate_regex_invalid_syntax() {
            let pattern = "^(abc";
            let _ = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_invalid_characters() {
            let pattern = "^ghO";
            let _ = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        fn test_generate_regex_starts_with_e() {
            let pattern = "^e";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("e"),
                "Address should start with 'e': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_contains_11() {
            let pattern = ".*11.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("11"),
                "Address should contain '11': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_contains_22() {
            let pattern = ".*22.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("22"),
                "Address should contain '22': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_ends_with_t() {
            let pattern = "t$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("t"),
                "Address should end with 't': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_complex_sequence() {
            let pattern = "11.*22";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("11") && address.contains("22"),
                "Address should contain '11' followed by '22': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_complex_pattern() {
            let pattern = "^e.*9.*t$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("e"),
                "Address should start with 'e': {}",
                address
            );
            assert!(
                address.contains("9"),
                "Address should contain '9': {}",
                address
            );
            assert!(
                address.ends_with("t"),
                "Address should end with 't': {}",
                address
            );
        }
    }
}
