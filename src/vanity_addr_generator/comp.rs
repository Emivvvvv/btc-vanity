use memx::{memeq, memmem};

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

/// Performs a case-insensitive prefix match.
///
/// # Arguments
/// - `data`: The target byte slice to check.
/// - `pattern`: The prefix byte slice to match against.
///
/// # Returns
/// - `true` if the beginning of `data` matches `pattern` (case-insensitively).
/// - `false` otherwise.
#[inline(always)]
pub fn eq_prefix_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    if data.len() < pattern.len() {
        return false;
    }
    
    for i in 0..pattern.len() {
        let a = data[i];
        let b = pattern[i];

        // Convert `a` to lowercase if it is an uppercase ASCII letter
        let a = if a.is_ascii_uppercase() {
            a | 0b00100000
        } else {
            a
        };

        if a != b {
            return false;
        }
    }

    true
}

/// Performs a case-insensitive suffix match.
///
/// # Arguments
/// - `data`: The target byte slice to check.
/// - `pattern`: The suffix byte slice to match against.
///
/// # Returns
/// - `true` if the end of `data` matches `pattern` (case-insensitively).
/// - `false` otherwise.
#[inline(always)]
pub fn eq_suffix_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    if data.len() < pattern.len() {
        return false;
    }
    
    let start = data.len() - pattern.len();
    for i in 0..pattern.len() {
        let a = data[start + i];
        let b = pattern[i];

        // Convert `a` to lowercase if it is an uppercase ASCII letter
        let a = if a.is_ascii_uppercase() {
            a | 0b00100000
        } else {
            a
        };

        if a != b {
            return false;
        }
    }

    true
}

/// Performs a case-insensitive substring match (anywhere match).
///
/// # Arguments
/// - `data`: The target byte slice to check.
/// - `pattern`: The byte slice to find within `data`.
///
/// # Returns
/// - `true` if `pattern` is found anywhere within `data` (case-insensitively).
/// - `false` otherwise.
#[inline(always)]
pub fn contains_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    if data.len() < pattern.len() {
        return false;
    }

    let pattern_len = pattern.len();
    let data_len = data.len();

    for start in 0..=(data_len - pattern_len) {
        let mut found = true;

        for i in 0..pattern_len {
            let a = data[start + i];
            let b = pattern[i];

            // Convert `a` to lowercase if it is an uppercase ASCII letter
            let a = if a.is_ascii_uppercase() {
                a | 0b00100000
            } else {
                a
            };

            if a != b {
                found = false;
                break; // Early exit on mismatch
            }
        }

        if found {
            return true; // Return early if a match is found
        }
    }

    false
}