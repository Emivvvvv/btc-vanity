//! # Custom Error Module For Better Error Handling And Output Styling
//!
//! This module is used for creating a better stylized custom errors for btc-vanity.

use thiserror::Error;

#[derive(Error, Debug)]
#[error("Custom Error: {0}")]
pub struct CustomError(pub &'static str);