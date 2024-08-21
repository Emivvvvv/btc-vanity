//! # Vanity Address Generation Module
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

use crate::error::VanitiyGeneretorError;
use crate::keys_and_address::{KeysAndAddress, KeysAndAddressString};
use bitcoin::secp256k1::{All, Secp256k1};
use std::sync::mpsc;
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
    /// Returns a Result type
    /// Returns OK if a vanity address found successfully with keys_and_address::KeysAndAddress struct
    /// Returns Err if the string if longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    pub fn generate(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<KeysAndAddressString, VanitiyGeneretorError> {
        if string.is_empty() {
            return Ok(KeysAndAddressString::generate_random());
        }
        if string.len() > 4 && fast_mode {
            return Err(VanitiyGeneretorError("You're asking for too much!\n\
            If you know this will take for a long time and really want to find something longer than 4 characters\n\
             disable fast mode with -df or --disable_fast flags."));
        }

        let is_base58 = string
            .chars()
            .find(|c| c == &'0' || c == &'I' || c == &'O' || c == &'l' || !c.is_alphanumeric());

        if is_base58.is_some() {
            return Err(VanitiyGeneretorError("Your input is not in base58. Don't include zero: '0', uppercase i: 'I', uppercase o: 'O', lowercase L: 'l'
            or any non-alphanumeric character in your input!"));
        }

        let secp256k1 = Secp256k1::new();
        Ok(SearchEngines::find_vanity_address_fast_engine(
            string,
            threads,
            case_sensitive,
            vanity_mode,
            secp256k1,
        ))
    }
}

/// An Empty Struct for a more structured code
///
/// At v1.0.0 the only search engine option is the new one.
/// ```Rust
/// fn find_vanity_address_fast_engine(
///         string: &str,
///         threads: u64,
///         case_sensitive: bool,
///         vanity_mode: VanityMode,
///         secp256k1: Secp256k1<All>) -> KeysAndAddressString  { ... }
/// ```
/// You can see the benchmark results between the old one and the new one
/// or you can just go back to v0.9.0 to have a look at the old search engine.
///
/// # Benchmark between old and new search engines.
///
/// As benchmark suggests new engine is a lot faster than the old one especially with string
/// searches that longer than 1 character.
///
/// Old engine's searches took 977.27 seconds total.
/// New engine's searches took 627.29 seconds total
///
/// Which means new engine is faster than the old one by ~1.58x (977.27 / 627.29)!
/// (I think this is a better calculation than the function suggests (~1.64x))
///
/// This test ran on a 8 cores m1 pro macbook pro 14 inch. Fans were on full blast mode.
///
///```bash
/// $ cargo test --test benchmark -- --nocapture --test-threads=1
/// ````
///
/// ```bash
///    Compiling btc-vanity v0.9.0 (/Users/emivvvvv/Documents/GitHub/btc-vanity)
///     Finished test [optimized + debuginfo] target(s) in 0.15s
///      Running tests/benchmark.rs (target/debug/deps/benchmark-981a0dc8e71e6fdd)
///
/// running 1 test
/// test benchmarks ...
/// Test settings ( threads: 16, case_sensititve: false, fast_mode: true, vanity_mode: Anywhere)
///
/// test string: e, test count: 200000
/// Finding 200000 vanity address took average: 163.35582925s with the old engine
/// Finding 200000 vanity address took average: 145.740485167s with the new engine
/// New engine is 1.1208678841902788x faster than the old one!
///
///
/// test string: mi, test count: 8000
/// Finding 8000 vanity address took average: 113.937177833s with the old engine
/// Finding 8000 vanity address took average: 50.817601375s with the new engine
/// New engine is 2.2420809867081215x faster than the old one!
///
///
/// test string: vvv, test count: 1000
/// Finding 1000 vanity address took average: 113.463775042s with the old engine
/// Finding 1000 vanity address took average: 64.925782208s with the new engine
/// New engine is 1.7475919609023864x faster than the old one!
///
///
/// test string: Emiv, test count: 100
/// Finding 100 vanity address took average: 445.538857334s with the old engine
/// Finding 100 vanity address took average: 266.378989417s with the new engine
/// New engine is 1.6725750717393713x faster than the old one!
///
///
/// test string: 3169, test count: 10
/// Finding 10 vanity address took average: 140.982839292s with the old engine
/// Finding 10 vanity address took average: 99.419238167s with the new engine
/// New engine is 1.4180639672090758x faster than the old one!
///
/// Final result. New engine is 1.640235974149847x faster than the old one overall!
/// ok
///
/// test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1604.56s
/// ```
pub struct SearchEngines;

impl SearchEngines {
    /// The faster search engine which is faster by ~1.58x than the old one!
    /// Search for the vanity address with given threads.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements it sends
    /// the keys_and_address::KeysAndAddress struct wia std::sync::mpsc channel and find_vanity_address function kills all of the other
    /// threads and closes the channel and returns the found KeysAndAddress struct that includes
    /// key pair and the desired address.
    fn find_vanity_address_fast_engine(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        secp256k1: Secp256k1<All>,
    ) -> KeysAndAddressString {
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
                        && sender
                            .send(KeysAndAddressString::fast_engine_get(
                                keys_and_address.get_private_key(),
                                *keys_and_address.get_public_key(),
                                address.to_string(),
                            ))
                            .is_err()
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
}
