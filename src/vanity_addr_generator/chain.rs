use crate::keys_and_address::{KeyPairGenerator, BitcoinKeyPair, EthereumKeyPair};
use crate::error::VanityError;
use crate::utils::is_valid_base58_char;

/// Allowed "meta" characters for a simple subset of regex usage.
const ALLOWED_REGEX_META: &[char] = &[
    '^', '$', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '-',
];

pub trait Chain: KeyPairGenerator + Send {
    /// Validates input for the specific chain.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), VanityError>;

    /// Validates regex input for the specific chain.
    fn validate_regex_input(regex_str: &str) -> Result<(), VanityError>;
}

impl Chain for BitcoinKeyPair {
    /// Checks all given information before passing to the vanity address finder function.
    /// 1) If length > 4 and `fast_mode` is true, reject (too long).
    /// 2) If any character is not base58, reject.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), VanityError> {
        // 1) If length > 4 and `fast_mode` is true, reject (too long).
        if string.len() > 4 && fast_mode {
            return Err(VanityError::FastModeEnabled);
        }

        // 2) If any character is not base58, reject.
        if string.chars().any(|c| !is_valid_base58_char(c)) {
            return Err(VanityError::InputNotBase58);
        }

        Ok(())
    }

    /// Checks regex input before passing to the vanity address finder function.
    /// 1) If it's a recognized regex meta character, allow it.
    /// 2) If it's alphanumeric, ensure it's valid base58
    /// 3) Neither a recognized meta char, nor a valid base58 alphanumeric => reject
    fn validate_regex_input(regex_str: &str) -> Result<(), VanityError> {
        // For each character in the pattern:
        for c in regex_str.chars() {
            // 1) If it's a recognized regex meta character, allow it.
            if ALLOWED_REGEX_META.contains(&c) {
                continue;
            }

            // 2) If it's alphanumeric, ensure it's valid base58
            if c.is_alphanumeric() {
                if !is_valid_base58_char(c) {
                    return Err(VanityError::RegexNotBase58);
                }
            } else {
                // 3) Neither a recognized meta char, nor a valid base58 alphanumeric => reject
                return Err(VanityError::InvalidRegex);
            }
        }

        Ok(())
    }
}

impl Chain for EthereumKeyPair {
    /// Checks all given information before passing to the Ethereum vanity address finder function.
    /// 1) If length > 12 and `fast_mode` is true, reject (too long).
    /// 2) If any character is not base16, reject.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), VanityError> {
        // 1) If length > 12 and `fast_mode` is true, reject (too long).
        if string.len() > 12 && fast_mode {
            return Err(VanityError::FastModeEnabled);
        }

        // 2) If any character is not base16, reject.
        if string.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err(VanityError::InputNotBase16);
        }

        Ok(())
    }

    /// Checks regex input before passing to the Ethereum vanity address finder function.
    /// 1) If it's a recognized regex meta character, allow it.
    /// 2) If it's alphanumeric, ensure it's valid base16.
    /// 3) Neither a recognized meta char, nor a valid base16 alphanumeric => reject.
    fn validate_regex_input(regex_str: &str) -> Result<(), VanityError> {
        // For each character in the pattern:
        for c in regex_str.chars() {
            // 1) If it's a recognized regex meta character, allow it.
            if ALLOWED_REGEX_META.contains(&c) {
                continue;
            }

            // 2) If it's alphanumeric, ensure it's valid base16.
            if c.is_alphanumeric() {
                if !c.is_ascii_hexdigit() {
                    return Err(VanityError::RegexNotBase16);
                }
            } else {
                // 3) Neither a recognized meta char, nor a valid base16 alphanumeric => reject.
                return Err(VanityError::InvalidRegex);
            }
        }

        Ok(())
    }
}