//! # Vanity Chain Module
//!
//! This module defines the [VanityChain] trait, which provides chain-specific behavior for
//! generating vanity addresses. It supports `Bitcoin`, `Ethereum`, and `Solana` chains and handles:
//! - Input validation for both plain and regex patterns.
//! - Adjustments to inputs and patterns for chain-specific constraints.

use crate::error::VanityError;
use crate::keys_and_address::{BitcoinKeyPair, KeyPairGenerator};
#[cfg(feature = "ethereum")]
use crate::keys_and_address::EthereumKeyPair;
#[cfg(feature = "solana")]
use crate::keys_and_address::SolanaKeyPair;
use crate::VanityMode;

/// Maximum length constraints for fast mode and general input.
const BASE58_FAST_MODE_MAX: usize = 5;
const BASE58_MAX: usize = 25;
#[cfg(feature = "ethereum")]
const BASE16_FAST_MODE_MAX: usize = 16;
#[cfg(feature = "ethereum")]
const BASE16_MAX: usize = 40;

const ALLOWED_REGEX_META: &[char] = &[
    '^', '$', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '-',
];

/// The `VanityChain` trait defines chain-specific behavior for vanity address generation.
///
/// This trait is implemented for [BitcoinKeyPair], [EthereumKeyPair], and [SolanaKeyPair]
/// and provides default implementations for input validation and
/// adjustments for chain-specific constraints.
pub trait VanityChain: KeyPairGenerator + Send {
    /// Validates a plain input string for the chain.
    ///
    /// # Arguments
    /// - `string`: The input string to validate.
    /// - `fast_mode`: Whether fast mode is enabled.
    ///
    /// # Returns
    /// - `Ok(())` if the input is valid.
    /// - `Err(VanityError)` if the input is invalid.
    ///
    /// # Behavior
    /// - Rejects inputs that exceed the length limit in fast mode.
    /// - Rejects inputs that exceed the max length limit.
    /// - Ensures all characters are valid for the specific chain.
    /// - Makes additional chain-specific checks if needed.
    fn validate_input(
        string: &str,
        fast_mode: bool,
        case_sensitive: bool,
    ) -> Result<(), VanityError>;

    /// Validates a regex input string for the chain.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern string to validate.
    ///
    /// # Returns
    /// - `Ok(())` if the regex is valid.
    /// - `Err(VanityError)` if the regex contains invalid characters.
    ///
    /// # Behavior
    /// - Allows recognized regex meta characters.
    /// - Ensures alphanumeric characters are valid for the chain.
    /// - Rejects characters that are neither valid regex meta nor chain-specific valid characters.
    fn validate_regex_pattern(regex_str: &str) -> Result<(), VanityError>;

    /// Adjusts a plain input string for chain-specific requirements.
    ///
    /// # Arguments
    /// - `input`: The input string to adjust.
    /// - `_vanity_mode`: The vanity mode to apply (e.g., prefix adjustment).
    ///
    /// # Returns
    /// - A chain-adjusted input string.
    fn adjust_input(input: &str, _vanity_mode: VanityMode) -> String {
        input.to_string()
    }

    /// Adjusts a regex pattern string for chain-specific requirements.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern string to adjust.
    ///
    /// # Returns
    /// - A chain-adjusted regex pattern string.
    fn adjust_regex_pattern(regex_str: &str) -> String {
        regex_str.to_string()
    }
}

/// Validates a Base58 input string for Bitcoin and Solana chains.
///
/// # Arguments
/// - `string`: The input string to validate.
/// - `fast_mode`: Whether fast mode is enabled.
///
/// # Returns
/// - `Ok(())` if the input is valid.
/// - `Err(VanityError)` if the input is invalid.
///
/// # Behavior
/// - Rejects inputs that exceed the max length limit or are invalid Base58 strings.
fn validate_base58_input(string: &str, fast_mode: bool) -> Result<(), VanityError> {
    if string.len() > BASE58_FAST_MODE_MAX && fast_mode {
        return Err(VanityError::FastModeEnabled);
    }

    if string.len() > BASE58_MAX {
        return Err(VanityError::RequestTooLong);
    }

    if string.chars().any(|c| !is_valid_base58_char(c)) {
        return Err(VanityError::InputNotBase58);
    }

    Ok(())
}

/// Validates a regex pattern for Base58 for Bitcoin and Solana chains.
///
/// # Arguments
/// - `regex_str`: The regex pattern string to validate.
///
/// # Returns
/// - `Ok(())` if the regex is valid.
/// - `Err(VanityError)` if the regex contains invalid characters.
///
/// # Behavior
/// - Allows regex meta characters and Base58 alphanumeric characters.
/// - Rejects invalid regex meta characters and non-Base58 characters.
fn validate_base58_regex_pattern(regex_str: &str) -> Result<(), VanityError> {
    // For each character in the pattern:
    for c in regex_str.chars() {
        // If it's a recognized regex meta character, allow it.
        if ALLOWED_REGEX_META.contains(&c) {
            continue;
        }

        // If it's alphanumeric, ensure it's valid base58
        if c.is_alphanumeric() {
            if !is_valid_base58_char(c) {
                return Err(VanityError::RegexNotBase58);
            }
        } else {
            // Neither a recognized meta char, nor a valid base58 alphanumeric => reject
            return Err(VanityError::InvalidRegex);
        }
    }

    Ok(())
}

impl VanityChain for BitcoinKeyPair {
    /// Validates a Base58 input string for Bitcoin-specific vanity address generation.
    ///
    /// # Arguments
    /// - `string`: The input string to validate.
    /// - `fast_mode`: Whether fast mode is enabled.
    /// - `_case_sensitive`: Unused for Bitcoin and Solana as they are case-insensitive.
    ///
    /// # Returns
    /// - `Ok(())` if the input is valid.
    /// - `Err(VanityError)` if the input is invalid.
    ///
    /// # Behavior
    /// - Rejects inputs that exceed the max length limit or the fast mode length limit.
    /// - Ensures all characters are valid Base58 characters.
    ///
    /// # Implementation Notes
    /// - The validation relies on the `validate_base58_input` helper function, which encapsulates
    fn validate_input(
        string: &str,
        fast_mode: bool,
        _case_sensitive: bool,
    ) -> Result<(), VanityError> {
        validate_base58_input(string, fast_mode)
    }

    /// Validates a regex pattern for Bitcoin-specific vanity address generation.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern string to validate.
    ///
    /// # Returns
    /// - `Ok(())` if the regex is valid.
    /// - `Err(VanityError)` if the regex contains invalid characters.
    ///
    /// # Behavior
    /// - Allows recognized regex meta characters.
    /// - Ensures all alphanumeric characters in the regex are valid Base58.
    /// - Rejects invalid regex meta characters or non-Base58 characters.
    ///
    /// # Implementation Notes
    /// - The validation relies on the `validate_base58_regex_pattern` helper function, which
    ///   encapsulates the Base58-specific regex validation logic.
    fn validate_regex_pattern(regex_str: &str) -> Result<(), VanityError> {
        validate_base58_regex_pattern(regex_str)
    }

    /// Adjusts the input string for Bitcoin-specific vanity address generation.
    ///
    /// # Arguments
    /// - `input`: The input string to adjust.
    /// - `vanity_mode`: The vanity mode (e.g., `Prefix`, `Suffix`).
    ///
    /// # Returns
    /// - The adjusted input string, with '1' prepended if in `Prefix` mode.
    ///
    /// # Behavior
    /// - Bitcoin addresses must always start with a '1'. If the `VanityMode` is `Prefix`,
    ///   the method prepends '1' to the input string.
    /// - Other modes (e.g., Suffix or Anywhere) do not modify the input.
    fn adjust_input(input: &str, vanity_mode: VanityMode) -> String {
        match vanity_mode {
            VanityMode::Prefix => format!("1{input}"),
            _ => input.to_string(),
        }
    }

    /// Adjusts a regex pattern for Bitcoin-specific vanity address generation.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern to adjust.
    ///
    /// # Returns
    /// - The adjusted regex pattern, ensuring prefix patterns start with `1`.
    ///
    /// # Example
    /// - Input: `^abc` => Output: `^1abc`
    ///
    /// # Behavior
    /// - For regex patterns, if the pattern starts with `^` (indicating a prefix match) but does not start
    ///   with `^1`, the method inserts `1` after `^` to ensure the regex respects Bitcoin's address format.
    fn adjust_regex_pattern(regex_str: &str) -> String {
        let mut pattern_str = regex_str.to_string();
        if pattern_str.starts_with('^') && !pattern_str.starts_with("^1") {
            pattern_str.insert(1, '1');
        }
        pattern_str
    }
}

#[cfg(feature = "ethereum")]
impl VanityChain for EthereumKeyPair {
    /// Validates a Base16 input string for Ethereum-specific vanity address generation.
    ///
    /// # Arguments
    /// - `string`: The input string to validate.
    /// - `fast_mode`: Whether fast mode is enabled.
    /// - `case_sensitive`: Whether the matching is case-sensitive. **Ethereum does not support case-sensitive inputs.**
    ///
    /// # Returns
    /// - `Ok(())` if the input is valid.
    /// - `Err(VanityError)` if the input is invalid.
    ///
    /// # Behavior
    /// - Rejects inputs if `case_sensitive` is enabled.
    /// - Rejects inputs that exceed the length limit in fast mode.
    /// - Rejects inputs that exceed the max length limit.
    /// - Ensures all characters are valid Base16 (hexadecimal) characters.
    fn validate_input(
        string: &str,
        fast_mode: bool,
        case_sensitive: bool,
    ) -> Result<(), VanityError> {
        if case_sensitive {
            return Err(VanityError::EthereumCaseSensitiveIsNotSupported);
        }

        if string.len() > BASE16_FAST_MODE_MAX && fast_mode {
            return Err(VanityError::FastModeEnabled);
        }

        if string.len() > BASE16_MAX {
            return Err(VanityError::RequestTooLong);
        }

        // If any character is not base16, reject.
        if string.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err(VanityError::InputNotBase16);
        }

        Ok(())
    }

    /// Validates a regex pattern for Base16.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern to validate.
    ///
    /// # Returns
    /// - `Ok(())` if the pattern is valid.
    /// - `Err(VanityError)` if the pattern is invalid.
    ///
    /// # Errors
    /// - Returns `VanityError::RegexNotBase16` for invalid Base16 characters.
    /// - Returns `VanityError::InvalidRegex` for unrecognized characters.
    ///
    /// # Behavior
    /// - Allows recognized regex meta characters.
    /// - Ensures all alphanumeric characters in the regex are valid Base16 (hexadecimal).
    fn validate_regex_pattern(regex_str: &str) -> Result<(), VanityError> {
        // For each character in the pattern:
        for c in regex_str.chars() {
            // If it's a recognized regex meta character, allow it.
            if ALLOWED_REGEX_META.contains(&c) {
                continue;
            }

            // If it's alphanumeric, ensure it's valid base16.
            if c.is_alphanumeric() {
                if !c.is_ascii_hexdigit() {
                    return Err(VanityError::RegexNotBase16);
                }
            } else {
                // Neither a recognized meta char, nor a valid base16 alphanumeric => reject.
                return Err(VanityError::InvalidRegex);
            }
        }

        Ok(())
    }

    /// Adjusts a regex pattern for Ethereum-specific vanity address generation.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern to adjust.
    ///
    /// # Returns
    /// - The adjusted regex pattern, with `^0x` removed if present.
    ///
    /// # Example
    /// - Input: `^0xabc` => Output: `^abc`
    ///
    /// # Behavior:
    /// - For regex patterns, if the pattern starts with `^0x`, the `0x` is removed for vanity
    ///   address generation.
    fn adjust_regex_pattern(regex_str: &str) -> String {
        let mut pattern_str = regex_str.to_string().to_ascii_lowercase();
        if pattern_str.starts_with("^0x") {
            pattern_str = pattern_str.replacen("^0x", "^", 1);
        }
        pattern_str
    }
}

#[cfg(feature = "solana")]
impl VanityChain for SolanaKeyPair {
    /// Validates a Base58 input string for Solana-specific vanity address generation.
    ///
    /// # Arguments
    /// - `string`: The input string to validate.
    /// - `fast_mode`: Whether fast mode is enabled.
    /// - `_case_sensitive`: Unused for Bitcoin and Solana as they are case-insensitive.
    ///
    /// # Returns
    /// - `Ok(())` if the input is valid.
    /// - `Err(VanityError)` if the input is invalid.
    ///
    /// # Behavior
    /// - Rejects inputs that exceed the max length limit or the fast mode length limit.
    /// - Ensures all characters are valid Base58 characters.
    ///
    /// # Implementation Notes
    /// - The validation relies on the `validate_base58_input` helper function, which encapsulates
    fn validate_input(
        string: &str,
        fast_mode: bool,
        _case_sensitive: bool,
    ) -> Result<(), VanityError> {
        validate_base58_input(string, fast_mode)
    }

    /// Validates a regex pattern for Solana-specific vanity address generation.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern string to validate.
    ///
    /// # Returns
    /// - `Ok(())` if the regex is valid.
    /// - `Err(VanityError)` if the regex contains invalid characters.
    ///
    /// # Behavior
    /// - Allows recognized regex meta characters.
    /// - Ensures all alphanumeric characters in the regex are valid Base58.
    /// - Rejects invalid regex meta characters or non-Base58 characters.
    ///
    /// # Implementation Notes
    /// - The validation relies on the `validate_base58_regex_pattern` helper function, which
    ///   encapsulates the Base58-specific regex validation logic.
    fn validate_regex_pattern(regex_str: &str) -> Result<(), VanityError> {
        validate_base58_regex_pattern(regex_str)
    }
}

/// Returns `true` if `c` is a valid Base58 character for Bitcoin and Solana chains.
///
/// # Arguments
/// - `c`: The character to validate.
///
/// # Returns
/// - `true` if the character is valid.
/// - `false` otherwise.
///
/// # Valid Characters
/// - Digits: `1-9`
/// - Uppercase letters (excluding `I` and `O`)
/// - Lowercase letters (excluding `l`)
pub fn is_valid_base58_char(c: char) -> bool {
    match c {
        // digits except 0
        '1'..='9' => true,
        // uppercase letters except I, O
        'A'..='H' | 'J'..='N' | 'P'..='Z' => true,
        // lowercase letters except l
        'a'..='k' | 'm'..='z' => true,
        _ => false,
    }
}

/// Represents supported blockchain chains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Chain {
    #[default]
    Bitcoin,
    Ethereum,
    Solana,
}

impl std::str::FromStr for Chain {
    type Err = String;

    /// Parses a string into a `Chain` variant.
    fn from_str(chain: &str) -> Result<Self, Self::Err> {
        match chain.to_lowercase().as_str() {
            "bitcoin" => Ok(Chain::Bitcoin),
            "ethereum" => Ok(Chain::Ethereum),
            "solana" => Ok(Chain::Solana),
            _ => Err(format!("Unsupported chain: {}", chain)),
        }
    }
}

impl std::fmt::Display for Chain {
    /// Converts a `Chain` variant to a string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Chain::Bitcoin => "bitcoin",
                Chain::Ethereum => "ethereum",
                Chain::Solana => "solana",
            }
        )
    }
}
