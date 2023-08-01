#![allow(unused_variables)]
use crate::keys_and_address::KeysAndAddress;
use crate::error::CustomError;

pub struct VanityAddr;

pub enum VanityMode {
    Prefix,
    Suffix,
    Anywhere,
}

impl VanityAddr {
    pub fn generate(
        string: String,
        mode: VanityMode,
        case_sensitive: bool,
        fast_mode: bool)
        -> Result<KeysAndAddress, CustomError> {
        if string.len() == 0 { return Ok( KeysAndAddress::generate_random()) }
        if string.len() > 5 && fast_mode {
            return Err(CustomError("You're asking for too much!\n\
            If you know this will take for a long time and really want to find something longer than 4 characters\n\
             disable fast mode with -df or --disable_fas flags."))
        }
        //check for base58( is_alphanumeric() - “0”, “I”, “O”, and “l”)
        match mode {
            VanityMode::Prefix => Ok(find_vanity_address_prefix(string, case_sensitive)),
            VanityMode::Suffix => Ok(find_vanity_address_suffix(string, case_sensitive)),
            VanityMode::Anywhere => Ok(find_vanity_address_anywhere(string, case_sensitive)),
        }
    }
}

// all will be in the same function in the near future.
fn find_vanity_address_prefix(string: String, case_sensitive: bool) -> KeysAndAddress {
    todo!()
}

fn find_vanity_address_suffix(string: String, case_sensitive: bool) -> KeysAndAddress {
    todo!()
}

fn find_vanity_address_anywhere(string: String, case_sensitive: bool) -> KeysAndAddress {
    todo!()
}

