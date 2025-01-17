//! # Utils Module
//!
//! This module is used in [crate::vanity_addr_generator] to check only contains Base58 characters or not.

/// Returns true if `c` is a valid Base58 character for (legacy) Bitcoin addresses.
///
/// Valid base58:
///   Digits except 0
///   Uppercase letters except I, O
///   Lowercase letters except l
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
