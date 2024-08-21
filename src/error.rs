//! # Custom Error Module For Better Error Handling And Output Styling
//!
//! This module is used for creating a better stylized custom errors for btc-vanity.

use thiserror::Error;

#[derive(Error, Debug)]
#[error("File error: {0}")]
pub struct FileError(pub String);

impl From<std::io::Error> for FileError {
    fn from(io_err: std::io::Error) -> Self {
        FileError(io_err.to_string())
    }
}

#[derive(Error, Debug)]
#[error("Keys and address error: {0}")]
pub struct KeysAndAdressError(pub &'static str);

#[derive(Error, Debug)]
#[error("Vanity address generator error: {0}")]
pub struct VanitiyGeneretorError(pub &'static str);

impl From<KeysAndAdressError> for VanitiyGeneretorError {
    fn from(keys_and_address_err: KeysAndAdressError) -> Self {
        VanitiyGeneretorError(keys_and_address_err.0)
    }
}
