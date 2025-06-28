use memx::{memeq, memmem};

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

    // For medium patterns (5-16 bytes), use optimized Boyer-Moore
    if pattern_len <= 16 {
        // Create bad character table
        let mut bad_char = [pattern_len; 256];
        for (i, &byte) in pattern.iter().enumerate() {
            bad_char[byte as usize] = pattern_len - 1 - i;
        }

        let mut pos = 0;
        while pos <= data_len - pattern_len {
            let mut j = pattern_len;
            let mut found_mismatch = false;

            // Check from the end of the pattern
            while j > 0 {
                j -= 1;
                let data_char = data[pos + j];
                let pattern_char = pattern[j];
                let data_lower = ASCII_LOWERCASE[data_char as usize];
                
                if data_lower != pattern_char {
                    found_mismatch = true;
                    break;
                }
            }

            if j == 0 && !found_mismatch {
                return true; // Match found
            }

            // Use bad character heuristic to skip positions
            let bad_char_skip = if pos + pattern_len - 1 < data_len {
                bad_char[ASCII_LOWERCASE[data[pos + pattern_len - 1] as usize] as usize]
            } else {
                1
            };
            pos += bad_char_skip.max(1);
        }

        return false;
    }

    // For very small (2-4 bytes) or very large (more than 16 bytes) patterns, use simple scan
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
}
