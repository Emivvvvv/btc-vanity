use thiserror::Error;

#[derive(Error, Debug)]
#[error("Custom Error: {0}")]
pub struct CustomError(pub &'static str);