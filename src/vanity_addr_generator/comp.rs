use memx::{memeq, memmem};

use crate::VanityMode;

/// Lookup table for ASCII case conversion
static ASCII_LOWERCASE: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 97, 98, 99, 100, 101, 102, 103,
    104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122,
    91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
    112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130,
    131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149,
    150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168,
    169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187,
    188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206,
    207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225,
    226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244,
    245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
];

/// Matching state prepared once and shared by all CPU workers in a search.
pub struct CompiledPattern {
    pattern: Box<[u8]>,
    case_sensitive: bool,
    mode: VanityMode,
    bad_char: Option<Box<[usize; 256]>>,
}

impl CompiledPattern {
    pub fn new(pattern: &[u8], case_sensitive: bool, mode: VanityMode) -> Self {
        let pattern: Box<[u8]> = if case_sensitive {
            pattern.into()
        } else {
            pattern
                .iter()
                .map(|byte| ASCII_LOWERCASE[*byte as usize])
                .collect::<Vec<_>>()
                .into_boxed_slice()
        };
        let bad_char = if !case_sensitive
            && matches!(mode, VanityMode::Anywhere)
            && (5..=16).contains(&pattern.len())
        {
            Some(Box::new(build_bad_char_table(&pattern)))
        } else {
            None
        };

        Self {
            pattern,
            case_sensitive,
            mode,
            bad_char,
        }
    }

    #[inline(always)]
    pub fn matches(&self, address: &[u8]) -> bool {
        if self.case_sensitive {
            return match self.mode {
                VanityMode::Prefix => eq_prefix_memx(address, &self.pattern),
                VanityMode::Suffix => eq_suffix_memx(address, &self.pattern),
                VanityMode::Anywhere => contains_memx(address, &self.pattern),
                VanityMode::Regex => false,
            };
        }

        match self.mode {
            VanityMode::Prefix => eq_prefix_case_insensitive(address, &self.pattern),
            VanityMode::Suffix => eq_suffix_case_insensitive(address, &self.pattern),
            VanityMode::Anywhere => match self.bad_char.as_deref() {
                Some(bad_char) => {
                    contains_case_insensitive_with_table(address, &self.pattern, bad_char)
                }
                None => contains_case_insensitive_simple(address, &self.pattern),
            },
            VanityMode::Regex => false,
        }
    }
}

/// Performs a case-sensitive prefix match using the `memx` crate.
///
/// # Arguments
/// - `addr`: The target byte slice to check.
/// - `pat`: The prefix byte slice to match against.
///
/// # Returns
/// - `true` if the beginning of `addr` matches `pat`.
/// - `false` otherwise.
#[inline(always)]
pub fn eq_prefix_memx(addr: &[u8], pat: &[u8]) -> bool {
    if addr.len() < pat.len() {
        return false;
    }

    memeq(&addr[..pat.len()], pat)
}

/// Performs a case-sensitive suffix match using the `memx` crate.
///
/// # Arguments
/// - `addr`: The target byte slice to check.
/// - `pat`: The suffix byte slice to match against.
///
/// # Returns
/// - `true` if the end of `addr` matches `pat`.
/// - `false` otherwise.
#[inline(always)]
pub fn eq_suffix_memx(addr: &[u8], pat: &[u8]) -> bool {
    if addr.len() < pat.len() {
        return false;
    }

    let start = addr.len() - pat.len();
    memeq(&addr[start..], pat)
}

/// Performs a case-sensitive substring match (anywhere match) using the `memx` crate.
///
/// # Arguments
/// - `addr`: The target byte slice to check.
/// - `pat`: The byte slice to find within `addr`.
///
/// # Returns
/// - `true` if `pat` is found anywhere within `addr`.
/// - `false` otherwise.
#[inline(always)]
pub fn contains_memx(addr: &[u8], pat: &[u8]) -> bool {
    memmem(addr, pat).is_some()
}

/// Simple, fast case-insensitive prefix match.
///
/// # Arguments
/// - `data`: The target byte slice to check.
/// - `pattern`: The prefix byte slice to match against (should be lowercase).
///
/// # Returns
/// - `true` if the beginning of `data` matches `pattern` (case-insensitively).
/// - `false` otherwise.
#[inline(always)]
pub fn eq_prefix_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    let pattern_len = pattern.len();
    if data.len() < pattern_len {
        return false;
    }

    if pattern_len == 0 {
        return true;
    }

    // Simple, efficient byte-by-byte comparison with lookup table
    for i in 0..pattern_len {
        if ASCII_LOWERCASE[data[i] as usize] != pattern[i] {
            return false;
        }
    }
    true
}

/// Simple, fast case-insensitive suffix match.
///
/// # Arguments
/// - `data`: The target byte slice to check.
/// - `pattern`: The suffix byte slice to match against (should be lowercase).
///
/// # Returns
/// - `true` if the end of `data` matches `pattern` (case-insensitively).
/// - `false` otherwise.
#[inline(always)]
pub fn eq_suffix_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    let pattern_len = pattern.len();
    if data.len() < pattern_len {
        return false;
    }

    if pattern_len == 0 {
        return true;
    }

    let start = data.len() - pattern_len;

    // Simple, efficient byte-by-byte comparison with lookup table
    for i in 0..pattern_len {
        if ASCII_LOWERCASE[data[start + i] as usize] != pattern[i] {
            return false;
        }
    }
    true
}

/// High-performance case-insensitive substring search with adaptive algorithm selection.
/// Uses different algorithms based on pattern length for optimal performance.
///
/// # Arguments
/// - `data`: The target byte slice to check.
/// - `pattern`: The byte slice to find within `data` (should be lowercase).
///
/// # Returns
/// - `true` if `pattern` is found anywhere within `data` (case-insensitively).
/// - `false` otherwise.
#[inline(always)]
#[cfg(test)]
pub fn contains_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    let data_len = data.len();
    let pattern_len = pattern.len();

    if data_len < pattern_len {
        return false;
    }

    if pattern_len == 0 {
        return true;
    }

    // Fast path for single character search - our biggest optimization win
    if pattern_len == 1 {
        let target = pattern[0];
        return data
            .iter()
            .any(|&byte| ASCII_LOWERCASE[byte as usize] == target);
    }

    // For medium patterns (5-16 bytes), use optimized Boyer-Moore.
    if pattern_len <= 16 {
        let bad_char = build_bad_char_table(pattern);
        return contains_case_insensitive_with_table(data, pattern, &bad_char);
    }

    contains_case_insensitive_simple(data, pattern)
}

fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
    let pattern_len = pattern.len();
    let mut bad_char = [pattern_len; 256];
    for (index, byte) in pattern.iter().copied().enumerate() {
        bad_char[byte as usize] = pattern_len - 1 - index;
    }
    bad_char
}

#[inline(always)]
fn contains_case_insensitive_with_table(
    data: &[u8],
    pattern: &[u8],
    bad_char: &[usize; 256],
) -> bool {
    let data_len = data.len();
    let pattern_len = pattern.len();
    let mut pos = 0;

    while pos <= data_len - pattern_len {
        let mut index = pattern_len;
        let mut found_mismatch = false;

        while index > 0 {
            index -= 1;
            if ASCII_LOWERCASE[data[pos + index] as usize] != pattern[index] {
                found_mismatch = true;
                break;
            }
        }

        if index == 0 && !found_mismatch {
            return true;
        }

        let trailing_byte = ASCII_LOWERCASE[data[pos + pattern_len - 1] as usize];
        pos += bad_char[trailing_byte as usize].max(1);
    }

    false
}

#[inline(always)]
fn contains_case_insensitive_simple(data: &[u8], pattern: &[u8]) -> bool {
    let data_len = data.len();
    let pattern_len = pattern.len();

    if data_len < pattern_len {
        return false;
    }
    if pattern_len == 0 {
        return true;
    }
    if pattern_len == 1 {
        let target = pattern[0];
        return data
            .iter()
            .any(|byte| ASCII_LOWERCASE[*byte as usize] == target);
    }

    // For very small (2-4 bytes) or very large patterns, use a simple scan.
    for start in 0..=(data_len - pattern_len) {
        let mut matches = true;
        for i in 0..pattern_len {
            let data_char = data[start + i];
            let pattern_char = pattern[i];
            let data_lower = ASCII_LOWERCASE[data_char as usize];

            if data_lower != pattern_char {
                matches = false;
                break;
            }
        }
        if matches {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VanityMode;

    #[test]
    fn test_case_insensitive_contains() {
        let address = "abcDEF123";
        let pattern = "abc";
        let address_bytes = address.as_bytes();
        let pattern_bytes = pattern.as_bytes();

        let result = contains_case_insensitive(address_bytes, pattern_bytes);
        let contains_result = address.to_lowercase().contains(pattern);

        assert_eq!(result, contains_result);
        assert!(result);
    }

    #[test]
    fn test_case_insensitive_contains_hex() {
        // Test with Ethereum-like hex addresses
        let address = "a1b2c3d4e5f6789abcdef";
        let pattern = "abc";
        let address_bytes = address.as_bytes();
        let pattern_bytes = pattern.as_bytes();

        let result = contains_case_insensitive(address_bytes, pattern_bytes);
        let contains_result = address.to_lowercase().contains(pattern);

        assert_eq!(result, contains_result);
        assert!(result);
    }

    #[test]
    fn test_case_insensitive_contains_no_match() {
        let address = "2091ab99a2e6bcd34293eb76aafb55dab7ae2de1";
        let pattern = "abc";
        let address_bytes = address.as_bytes();
        let pattern_bytes = pattern.as_bytes();

        let result = contains_case_insensitive(address_bytes, pattern_bytes);
        let contains_result = address.to_lowercase().contains(pattern);

        assert_eq!(result, contains_result);
        assert!(!result);
    }

    #[test]
    fn compiled_pattern_matches_existing_matchers() {
        let addresses: [&[u8]; 3] = [
            b"1Emiv7YwS2dQx9KpR4nT6uV8zA3cF5gH",
            b"aB12cD34eF56aB78cD90eF12aB34cD56eF78aB90",
            b"7YwS2dQx9KpR4nT6uV8zA3cF5gH1Emiv",
        ];

        for address in addresses {
            let mut patterns = vec![
                Vec::new(),
                address[..1].to_vec(),
                address[..2].to_vec(),
                address[..4].to_vec(),
                address[..5].to_vec(),
                address[..16].to_vec(),
                address[..20].to_vec(),
                b"not-present-pattern".to_vec(),
            ];
            patterns.push(address[address.len() - 5..].to_vec());

            for pattern in patterns {
                for case_sensitive in [true, false] {
                    let lower_pattern = pattern
                        .iter()
                        .map(|byte| byte.to_ascii_lowercase())
                        .collect::<Vec<_>>();

                    for mode in [VanityMode::Prefix, VanityMode::Suffix, VanityMode::Anywhere] {
                        let expected = if case_sensitive {
                            match mode {
                                VanityMode::Prefix => eq_prefix_memx(address, &pattern),
                                VanityMode::Suffix => eq_suffix_memx(address, &pattern),
                                VanityMode::Anywhere => contains_memx(address, &pattern),
                                VanityMode::Regex => unreachable!(),
                            }
                        } else {
                            match mode {
                                VanityMode::Prefix => {
                                    eq_prefix_case_insensitive(address, &lower_pattern)
                                }
                                VanityMode::Suffix => {
                                    eq_suffix_case_insensitive(address, &lower_pattern)
                                }
                                VanityMode::Anywhere => {
                                    contains_case_insensitive(address, &lower_pattern)
                                }
                                VanityMode::Regex => unreachable!(),
                            }
                        };

                        let compiled = CompiledPattern::new(&pattern, case_sensitive, mode);
                        assert_eq!(
                            compiled.matches(address),
                            expected,
                            "address={}, pattern={}, case_sensitive={case_sensitive}, mode={mode:?}",
                            String::from_utf8_lossy(address),
                            String::from_utf8_lossy(&pattern),
                        );
                    }
                }
            }
        }
    }
}
