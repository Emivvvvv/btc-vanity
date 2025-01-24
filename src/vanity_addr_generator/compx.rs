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
