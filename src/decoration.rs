//! # Decoration Strings For Stylized Outputs
//!
//! This module is used for creating a better stylized outputs for btc-vanity.

use crate::vanity_addr_generator::VanityMode;

const VANITY_MODE_STR: [&str; 4] = [
    "has the prefix",
    "has the suffix",
    "has the string",
    "satisfies the given regex",
];
const CASE_SENSITIVITY_STR: [&str; 2] = ["(case sensitive)", "(case sensitivity disabled)"];

/// Returns string literals of desired mode's styled string.
pub fn get_decoration_strings<'a>(
    vanity_mode: VanityMode,
    is_case_sensitive: bool,
) -> (&'a str, &'a str) {
    // Sets vanity mode decoration string.
    let vanity_mode_str = match vanity_mode {
        VanityMode::Prefix => VANITY_MODE_STR[0],
        VanityMode::Suffix => VANITY_MODE_STR[1],
        VanityMode::Anywhere => VANITY_MODE_STR[2],
        VanityMode::Regex => VANITY_MODE_STR[3],
    };

    // Sets case sensitivity decoration string.
    let case_sensitive_str = match is_case_sensitive {
        true => CASE_SENSITIVITY_STR[0],
        false => CASE_SENSITIVITY_STR[1],
    };

    (vanity_mode_str, case_sensitive_str)
}
