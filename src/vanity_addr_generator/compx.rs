use memx::{memeq, memmem};

/// Case-sensitive prefix match using memx
#[inline(always)]
pub fn eq_prefix_memx(addr: &[u8], pat: &[u8]) -> bool {
    memeq(&addr[..pat.len()], pat)
}

/// Case-sensitive suffix match using memx
#[inline(always)]
pub fn eq_suffix_memx(addr: &[u8], pat: &[u8]) -> bool {
    let start = addr.len() - pat.len();
    memeq(&addr[start..], pat)
}

/// Case-sensitive "anywhere" (substring) check using memx
#[inline(always)]
pub fn contains_memx(addr: &[u8], pat: &[u8]) -> bool {
    memmem(addr, pat).is_some()
}

#[inline(always)]
pub unsafe fn eq_prefix_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    for i in 0..pattern.len() {
        let a = *data.get_unchecked(i);
        let b = *pattern.get_unchecked(i);

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

#[inline(always)]
pub unsafe fn eq_suffix_case_insensitive(data: &[u8], pattern: &[u8]) -> bool {
    let start = data.len() - pattern.len();
    for i in 0..pattern.len() {
        let a = *data.get_unchecked(start + i);
        let b = *pattern.get_unchecked(i);

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
