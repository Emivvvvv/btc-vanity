use std::io;
use log::error;
use thiserror::Error;

/// A unified error type that encapsulates all possible errors in the btc-vanity application.
#[derive(Error, Debug)]
pub enum VanityError {
    #[error("File error: {0}")]
    FileError(#[from] io::Error),

    #[error("Keys and address error: {0}")]
    KeysAndAddressError(&'static str),

    #[error("Vanity address generator error: {0}")]
    VanityGeneratorError(&'static str),

    #[error("Fast mode enabled, input is too long!")]
    FastModeEnabled,

    #[error("Input is not Base58 encoded!")]
    InputNotBase58,

    #[error("Input is not Base16 encoded!")]
    InputNotBase16,

    #[error("Invalid Regex!")]
    InvalidRegex,

    #[error("Regex is not Base58 encoded!")]
    RegexNotBase58,

    #[error("Regex is not Base16 encoded!")]
    RegexNotBase16,

    #[error("Request too long!")]
    RequestTooLong,

    #[error!("Case sensitive wallet generation is not supported for Ethereum!")]
    EthereumCaseSensitiveIsNotSupported,
}

impl From<KeysAndAddressError> for VanityError {
    fn from(keys_and_address_err: KeysAndAddressError) -> Self {
        VanityError::KeysAndAddressError(keys_and_address_err.0)
    }
}

impl From<VanityGeneratorError> for VanityError {
    fn from(vanity_err: VanityGeneratorError) -> Self {
        VanityError::VanityGeneratorError(vanity_err.0)
    }
}

/// Struct-based error types for backward compatibility or specific contexts.
#[derive(Error, Debug)]
#[error("Keys and address error: {0}")]
pub struct KeysAndAddressError(pub &'static str);

#[derive(Error, Debug)]
#[error("Vanity address generator error: {0}")]
pub struct VanityGeneratorError(pub &'static str);

impl From<KeysAndAddressError> for VanityGeneratorError {
    fn from(keys_and_address_err: KeysAndAddressError) -> Self {
        VanityGeneratorError(keys_and_address_err.0)
    }
}
