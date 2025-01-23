use crate::error::VanityError;
use crate::keys_and_address::{BitcoinKeyPair, EthereumKeyPair, KeyPairGenerator, SolanaKeyPair};
use crate::utils::is_valid_base58_char;
use crate::VanityMode;

const BASE58_FAST_MODE_MAX: usize = 4;
const BASE16_FAST_MODE_MAX: usize = 12;
const ALLOWED_REGEX_META: &[char] = &[
    '^', '$', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '-',
];

pub trait VanityChain: KeyPairGenerator + Send {
    /// Default implementation for Base58 (BTC and SOL) using chains.
    /// Checks all given information before passing to the vanity address finder function.
    /// 1) If length > 4 and `fast_mode` is true, reject (too long).
    /// 2) If any character is not base58, reject.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), VanityError> {
        // 1) If length > 4 (BASE58_FAST_MODE_MAX) and `fast_mode` is true, reject (too long).
        if string.len() > BASE58_FAST_MODE_MAX && fast_mode {
            return Err(VanityError::FastModeEnabled);
        }

        // 2) If any character is not base58, reject.
        if string.chars().any(|c| !is_valid_base58_char(c)) {
            return Err(VanityError::InputNotBase58);
        }

        Ok(())
    }

    /// Default implementation for Base58 (BTC and SOL) using chains.
    /// Checks regex input before passing to the vanity address finder function.
    /// 1) If it's a recognized regex meta character, allow it.
    /// 2) If it's alphanumeric, ensure it's valid base58
    /// 3) Neither a recognized meta char, nor a valid base58 alphanumeric => reject
    fn validate_regex_pattern(regex_str: &str) -> Result<(), VanityError> {
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

    fn adjust_input(input: &str, _vanity_mode: VanityMode) -> String {
        input.to_string()
    }

    fn adjust_regex_pattern(regex_str: &str) -> String {
        regex_str.to_string()
    }
}

impl VanityChain for BitcoinKeyPair {
    fn adjust_input(input: &str, vanity_mode: VanityMode) -> String {
        match vanity_mode {
            VanityMode::Prefix => format!("1{input}"),
            _ => input.to_string(),
        }
    }

    fn adjust_regex_pattern(regex_str: &str) -> String {
        let mut pattern_str = regex_str.to_string();
        if pattern_str.starts_with('^') && !pattern_str.starts_with("^1") {
            pattern_str.insert(1, '1');
        }
        pattern_str
    }
}

impl VanityChain for EthereumKeyPair {
    /// Checks all given information before passing to the Ethereum vanity address finder function.
    /// 1) If length > 12 and `fast_mode` is true, reject (too long).
    /// 2) If any character is not base16, reject.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), VanityError> {
        // 1) If length > 12 (BASE16_FAST_MODE_MAX) and `fast_mode` is true, reject (too long).
        if string.len() > BASE16_FAST_MODE_MAX && fast_mode {
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
    fn validate_regex_pattern(regex_str: &str) -> Result<(), VanityError> {
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

    fn adjust_regex_pattern(regex_str: &str) -> String {
        let mut pattern_str = regex_str.to_string();
        if pattern_str.starts_with("^0x") {
            pattern_str = pattern_str.replacen("^0x", "^", 1);
        }
        pattern_str
    }
}

impl VanityChain for SolanaKeyPair {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Chain {
    #[default]
    Bitcoin,
    Ethereum,
    Solana,
}

impl std::str::FromStr for Chain {
    type Err = String;

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
