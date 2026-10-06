//! # OpenCL Bitcoin prefix search
//!
//! A purpose-built GPU path for Bitcoin P2PKH (compressed) prefix searches.
//! Unlike the wgpu backend it never runs a scalar multiplication or base58
//! encode per candidate: each work-item walks consecutive keys with batched
//! affine additions, takes six public keys per point from the secp256k1
//! endomorphism and negation, and compares HASH160 against precomputed
//! ranges. Every GPU hit is re-derived and verified on the CPU before it is
//! returned.

use std::time::{Duration, Instant};

use bitcoin::hashes::{hash160, Hash};
use bitcoin::key::{PrivateKey, PublicKey};
use bitcoin::secp256k1::{rand, Secp256k1, SecretKey};
use bitcoin::{Address, Network};
use num_bigint::BigUint;
use opencl3::command_queue::CommandQueue;
use opencl3::context::Context;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};
use opencl3::kernel::{ExecuteKernel, Kernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE};
use opencl3::program::Program;
use opencl3::types::{cl_uint, cl_ulong, CL_BLOCKING};

use crate::keys_and_address::BitcoinKeyPair;

const KERNEL_SRC: &str = include_str!("opencl/btc_vanity.cl");
const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const MAX_HITS: usize = 4096;
const MAX_VARIANTS: usize = 1 << 16;

const ORDER_HEX: &str = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";
const LAMBDA_HEX: &str = "5363ad4cc05c30e0a5261c028812645a122e22ea20816678df02967c1b23bd72";
const LAMBDA2_HEX: &str = "ac9c52b33fa3cf1f5ad9e3fd77ed9ba4a880b9fc8ec739c2e0cfc810b51283ce";

/// Tuning for the OpenCL search.
#[derive(Debug, Clone, Copy)]
pub struct OclConfig {
    /// Concurrent work-items (each owns one walking key).
    pub items: usize,
    /// Points on each side of a work-item's centre per dispatch.
    pub group: usize,
    /// Zero-based index into the list of OpenCL GPU devices.
    pub device: usize,
}

impl Default for OclConfig {
    fn default() -> Self {
        Self {
            items: 65536,
            group: 256,
            device: 0,
        }
    }
}

/// Returns `true` when an OpenCL runtime exposes at least one GPU device.
pub fn opencl_device_available() -> bool {
    get_all_devices(CL_DEVICE_TYPE_GPU).is_ok_and(|devices| !devices.is_empty())
}

fn hex_big(s: &str) -> BigUint {
    BigUint::parse_bytes(s.as_bytes(), 16).expect("valid hex constant")
}

fn base58_value(s: &[u8]) -> Option<BigUint> {
    let mut v = BigUint::from(0u32);
    for c in s {
        let d = BASE58.iter().position(|b| b == c)?;
        v = v * 58u32 + d as u32;
    }
    Some(v)
}

/// Normalises the pattern to a full address prefix and expands case variants.
fn pattern_variants(pattern: &str, case_sensitive: bool) -> Result<Vec<String>, String> {
    let full = if pattern.starts_with('1') {
        pattern.to_string()
    } else {
        format!("1{pattern}")
    };
    let rest = &full[1..];
    if rest.is_empty() {
        return Err("pattern must contain at least one character after the leading '1'".into());
    }
    if rest.starts_with('1') {
        return Err("the OpenCL backend does not support patterns starting with '11'".into());
    }

    let mut variants = vec![String::from("1")];
    for c in rest.chars() {
        let mut options: Vec<char> = if case_sensitive {
            vec![c]
        } else {
            vec![c.to_ascii_lowercase(), c.to_ascii_uppercase()]
        };
        options.retain(|o| BASE58.contains(&(*o as u8)));
        options.dedup();
        if options.is_empty() {
            return Err(format!("'{c}' cannot appear in a base58 address"));
        }
        variants = variants
            .iter()
            .flat_map(|v| options.iter().map(move |o| format!("{v}{o}")))
            .collect();
        if variants.len() > MAX_VARIANTS {
            return Err("too many case variants; use --case-sensitive".into());
        }
    }
    Ok(variants)
}

/// Converts address prefixes into sorted, merged, inclusive ranges on the top
/// 64 bits of HASH160. A 25-byte P2PKH payload with one leading zero byte is
/// '1' followed by the base58 digits of the 192-bit value hash160||checksum.
fn prefix_ranges(variants: &[String]) -> Vec<(u64, u64)> {
    let min_v = BigUint::from(1u32) << 184;
    let max_v = (BigUint::from(1u32) << 192) - 1u32;
    let mut out = Vec::new();
    for v in variants {
        let digits = &v.as_bytes()[1..];
        let k = digits.len() as u32;
        let d = base58_value(digits).expect("validated base58");
        for len in k..=33u32 {
            let scale = BigUint::from(58u32).pow(len - k);
            let mut lo = &d * &scale;
            let mut hi = (&d + 1u32) * &scale - 1u32;
            let len_lo = BigUint::from(58u32).pow(len - 1);
            let len_hi = BigUint::from(58u32).pow(len) - 1u32;
            for bound in [&min_v, &len_lo] {
                if lo < *bound {
                    lo = bound.clone();
                }
            }
            for bound in [&max_v, &len_hi] {
                if hi > *bound {
                    hi = bound.clone();
                }
            }
            if lo <= hi {
                let to_u64 = |x: BigUint| -> u64 {
                    let digits = (x >> 128u32).to_u64_digits();
                    digits.first().copied().unwrap_or(0)
                };
                out.push((to_u64(lo), to_u64(hi)));
            }
        }
    }
    out.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (lo, hi) in out {
        match merged.last_mut() {
            Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    merged
}

/// A starting private key with its affine point as little-endian limbs.
type StartPoint = (BigUint, [u32; 8], [u32; 8]);

fn limbs_le(be32: &[u8]) -> [u32; 8] {
    let mut limbs = [0u32; 8];
    for (i, limb) in limbs.iter_mut().enumerate() {
        let off = 32 - 4 * (i + 1);
        *limb = u32::from_be_bytes([be32[off], be32[off + 1], be32[off + 2], be32[off + 3]]);
    }
    limbs
}

fn affine_of(secp: &Secp256k1<bitcoin::secp256k1::All>, sk: &SecretKey) -> ([u32; 8], [u32; 8]) {
    let pk = bitcoin::secp256k1::PublicKey::from_secret_key(secp, sk);
    let ser = pk.serialize_uncompressed();
    (limbs_le(&ser[1..33]), limbs_le(&ser[33..65]))
}

fn scalar_key(value: &BigUint) -> SecretKey {
    let bytes = value.to_bytes_be();
    let mut buf = [0u8; 32];
    buf[32 - bytes.len()..].copy_from_slice(&bytes);
    SecretKey::from_slice(&buf).expect("non-zero scalar below the group order")
}

fn ocl_err<E: std::fmt::Display>(context: &str) -> impl Fn(E) -> String + '_ {
    move |e| format!("OpenCL {context}: {e}")
}

/// Statistics from a fixed amount of OpenCL work.
#[derive(Debug, Clone, Copy)]
pub struct OclBenchStats {
    /// Public keys checked.
    pub keys: u64,
    /// Wall-clock seconds, including dispatch and readback overhead.
    pub seconds: f64,
    /// GPU hits whose CPU-derived address matched the pattern.
    pub verified_hits: u64,
    /// GPU hits that fell just outside the pattern at a range boundary.
    pub boundary_hits: u64,
    /// GPU hits whose HASH160 disagreed with the CPU derivation (always a bug).
    pub hash_mismatches: u64,
    /// Hits a correct implementation should report on average for this work.
    pub expected_hits: f64,
}

impl OclBenchStats {
    /// Throughput in keys per second.
    pub fn keys_per_second(&self) -> f64 {
        self.keys as f64 / self.seconds
    }
}

enum Outcome {
    Found(BitcoinKeyPair),
    Bench(OclBenchStats),
}

/// Searches for a compressed P2PKH address starting with `pattern` on an
/// OpenCL GPU. A missing leading '1' is added. Prints progress to stderr.
pub fn search_btc_prefix(
    pattern: &str,
    case_sensitive: bool,
    cfg: OclConfig,
) -> Result<BitcoinKeyPair, String> {
    match run(pattern, case_sensitive, cfg, None)? {
        Outcome::Found(pair) => Ok(pair),
        Outcome::Bench(_) => unreachable!("searches without a dispatch limit only end on a match"),
    }
}

/// Runs `dispatches` kernel launches for `pattern`, verifies every GPU hit on
/// the CPU, and returns throughput and correctness statistics. Short patterns
/// make useful correctness checks: many hits, each independently re-derived.
pub fn benchmark_btc_prefix(
    pattern: &str,
    case_sensitive: bool,
    cfg: OclConfig,
    dispatches: u64,
) -> Result<OclBenchStats, String> {
    match run(pattern, case_sensitive, cfg, Some(dispatches.max(1)))? {
        Outcome::Bench(stats) => Ok(stats),
        Outcome::Found(_) => unreachable!("benchmarks never stop at a match"),
    }
}

fn run(
    pattern: &str,
    case_sensitive: bool,
    cfg: OclConfig,
    bench: Option<u64>,
) -> Result<Outcome, String> {
    let verbose = bench.is_none();
    let variants = pattern_variants(pattern, case_sensitive)?;
    let target = variants[0].clone();
    let ranges = prefix_ranges(&variants);
    if ranges.is_empty() {
        return Err(format!("no P2PKH address can start with '{target}'"));
    }
    let probability: f64 = ranges
        .iter()
        .map(|(lo, hi)| (hi - lo) as f64 + 1.0)
        .sum::<f64>()
        / 2f64.powi(64);
    let expected = 1.0 / probability;

    let group = cfg.group.max(1);
    let items = cfg.items.max(64).next_multiple_of(64);
    let step = 2 * group as u64 + 1;
    let keys_per_dispatch = items as u64 * step * 6;

    // ---- device and program
    let devices = get_all_devices(CL_DEVICE_TYPE_GPU).map_err(ocl_err("device query"))?;
    let device_id = *devices
        .get(cfg.device)
        .ok_or_else(|| "no OpenCL GPU device found".to_string())?;
    let device = Device::new(device_id);
    let context = Context::from_device(&device).map_err(ocl_err("context"))?;
    #[allow(deprecated)]
    let queue = CommandQueue::create_default(&context, 0).map_err(ocl_err("queue"))?;
    let options = format!("-DGROUP={group} -cl-std=CL1.2");
    let program = Program::create_and_build_from_source(&context, KERNEL_SRC, &options)
        .map_err(|e| format!("OpenCL build failed: {e}"))?;
    let kernel = Kernel::create(&program, "btc_search").map_err(ocl_err("kernel"))?;

    if verbose {
        eprintln!(
            "OpenCL device: {} | {} work-items x {} keys/dispatch | {} range(s)",
            device.name().unwrap_or_default(),
            items,
            step * 6,
            ranges.len()
        );
        eprintln!(
            "Target '{}' ({}) | expected ~{:.3e} keys",
            target,
            if case_sensitive {
                "case sensitive"
            } else {
                "case insensitive"
            },
            expected
        );
    }

    // ---- starting keys and points
    let order = hex_big(ORDER_HEX);
    let lambdas = [
        BigUint::from(1u32),
        hex_big(LAMBDA_HEX),
        hex_big(LAMBDA2_HEX),
    ];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = items.div_ceil(threads);
    let mut keys: Vec<BigUint> = Vec::with_capacity(items);
    let mut centers = vec![0u32; 16 * items];
    let per_thread: Vec<Vec<StartPoint>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let count = chunk.min(items.saturating_sub(t * chunk));
                s.spawn(move || {
                    let secp = Secp256k1::new();
                    let mut rng = rand::thread_rng();
                    (0..count)
                        .map(|_| {
                            let sk = SecretKey::new(&mut rng);
                            let (x, y) = affine_of(&secp, &sk);
                            (BigUint::from_bytes_be(&sk.secret_bytes()), x, y)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("keygen thread"))
            .collect()
    });
    for (g, (k, x, y)) in per_thread.into_iter().flatten().enumerate() {
        for l in 0..8 {
            centers[l * items + g] = x[l];
            centers[(8 + l) * items + g] = y[l];
        }
        keys.push(k);
    }

    // table[i] = (i+1)G for i < group, table[group] = (2*group+1)G
    let secp = Secp256k1::new();
    let mut table = vec![0u32; 16 * (group + 1)];
    for i in 0..=group {
        let mult = if i < group { i as u64 + 1 } else { step };
        let (x, y) = affine_of(&secp, &scalar_key(&BigUint::from(mult)));
        table[i * 16..i * 16 + 8].copy_from_slice(&x);
        table[i * 16 + 8..i * 16 + 16].copy_from_slice(&y);
    }
    let range_words: Vec<cl_ulong> = ranges.iter().flat_map(|(lo, hi)| [*lo, *hi]).collect();

    // ---- buffers
    let mk = |flags, len: usize| unsafe {
        Buffer::<cl_uint>::create(&context, flags, len, std::ptr::null_mut())
    };
    let mut centers_buf = mk(CL_MEM_READ_WRITE, centers.len()).map_err(ocl_err("alloc"))?;
    let mut table_buf = mk(CL_MEM_READ_ONLY, table.len()).map_err(ocl_err("alloc"))?;
    let scratch_buf =
        mk(CL_MEM_READ_WRITE, (group + 1) * 8 * items).map_err(ocl_err("alloc scratch"))?;
    let mut out_buf = mk(CL_MEM_READ_WRITE, 1 + 4 * MAX_HITS).map_err(ocl_err("alloc"))?;
    let mut ranges_buf = unsafe {
        Buffer::<cl_ulong>::create(
            &context,
            CL_MEM_READ_ONLY,
            range_words.len(),
            std::ptr::null_mut(),
        )
    }
    .map_err(ocl_err("alloc"))?;
    unsafe {
        queue
            .enqueue_write_buffer(&mut centers_buf, CL_BLOCKING, 0, &centers, &[])
            .map_err(ocl_err("write"))?;
        queue
            .enqueue_write_buffer(&mut table_buf, CL_BLOCKING, 0, &table, &[])
            .map_err(ocl_err("write"))?;
        queue
            .enqueue_write_buffer(&mut ranges_buf, CL_BLOCKING, 0, &range_words, &[])
            .map_err(ocl_err("write"))?;
        queue
            .enqueue_write_buffer(&mut out_buf, CL_BLOCKING, 0, &[0u32], &[])
            .map_err(ocl_err("write"))?;
    }
    drop(centers);

    let nranges = ranges.len() as cl_uint;
    let max_hits = MAX_HITS as cl_uint;
    let mut hits = vec![0u32; 1 + 4 * MAX_HITS];
    let start = Instant::now();
    let mut last_report = Instant::now();
    let (mut verified, mut boundary, mut mismatches) = (0u64, 0u64, 0u64);

    for dispatch in 0u64.. {
        let event = unsafe {
            ExecuteKernel::new(&kernel)
                .set_arg(&centers_buf)
                .set_arg(&table_buf)
                .set_arg(&scratch_buf)
                .set_arg(&ranges_buf)
                .set_arg(&nranges)
                .set_arg(&out_buf)
                .set_arg(&max_hits)
                .set_global_work_size(items)
                .set_local_work_size(64)
                .enqueue_nd_range(&queue)
        }
        .map_err(ocl_err("dispatch"))?;
        event.wait().map_err(ocl_err("wait"))?;
        unsafe {
            queue
                .enqueue_read_buffer(&out_buf, CL_BLOCKING, 0, &mut hits, &[])
                .map_err(ocl_err("read"))?;
        }

        let count = (hits[0] as usize).min(MAX_HITS);
        if hits[0] > 0 {
            unsafe {
                queue
                    .enqueue_write_buffer(&mut out_buf, CL_BLOCKING, 0, &[0u32], &[])
                    .map_err(ocl_err("write"))?;
            }
        }
        for h in 0..count {
            let gid = hits[1 + 4 * h] as usize;
            let jidx = hits[2 + 4 * h] as u64;
            let flags = hits[3 + 4 * h];
            let gpu_h0 = hits[4 + 4 * h];
            // centre key for this dispatch, then offset within the group
            let base = (&keys[gid] + BigUint::from(dispatch) * step + BigUint::from(jidx) + &order
                - BigUint::from(group as u64))
                % &order;
            let mut key = (base * &lambdas[(flags & 3) as usize]) % &order;
            if flags & 4 != 0 {
                key = &order - key;
            }
            let sk = scalar_key(&key);
            let private_key = PrivateKey::new(sk, Network::Bitcoin);
            let public_key = PublicKey::from_private_key(&secp, &private_key);
            let address = Address::p2pkh(public_key, Network::Bitcoin).to_string();
            let h160 = hash160::Hash::hash(&public_key.to_bytes());
            let cpu_h0 = u32::from_le_bytes(h160.as_byte_array()[0..4].try_into().unwrap());
            let matches = if case_sensitive {
                address.starts_with(&target)
            } else {
                address.to_lowercase().starts_with(&target.to_lowercase())
            };
            if matches {
                verified += 1;
                if bench.is_none() {
                    let elapsed = start.elapsed().as_secs_f64();
                    let total = (dispatch + 1) * keys_per_dispatch;
                    eprintln!(
                        "\nMatch verified after {:.3e} keys in {:.1}s ({:.1} Mkeys/s)",
                        total as f64,
                        elapsed,
                        total as f64 / elapsed / 1e6
                    );
                    return Ok(Outcome::Found(BitcoinKeyPair {
                        private_key,
                        public_key,
                        comp_address: address,
                    }));
                }
            } else if cpu_h0 == gpu_h0 {
                boundary += 1;
            } else {
                mismatches += 1;
                eprintln!("warning: GPU hit failed CPU re-derivation (hash mismatch)");
            }
        }

        let done = dispatch + 1;
        if bench.is_some_and(|b| done >= b) {
            let total = done * keys_per_dispatch;
            return Ok(Outcome::Bench(OclBenchStats {
                keys: total,
                seconds: start.elapsed().as_secs_f64(),
                verified_hits: verified,
                boundary_hits: boundary,
                hash_mismatches: mismatches,
                expected_hits: total as f64 * probability,
            }));
        }
        if verbose && last_report.elapsed() >= Duration::from_secs(5) {
            last_report = Instant::now();
            let elapsed = start.elapsed().as_secs_f64();
            let total = done * keys_per_dispatch;
            eprintln!(
                "{:>8.0}s  {:.3e} keys  {:.1} Mkeys/s  P(found by now) {:.1}%",
                elapsed,
                total as f64,
                total as f64 / elapsed / 1e6,
                100.0 * (1.0 - (-(total as f64) / expected).exp())
            );
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_known_address() {
        // 1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH is the address of private key 1.
        let ranges = prefix_ranges(&pattern_variants("1BgGZ", true).unwrap());
        let decoded = bitcoin::base58::decode_check("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH").unwrap();
        let top = u64::from_be_bytes(decoded[1..9].try_into().unwrap());
        assert!(ranges.iter().any(|(lo, hi)| *lo <= top && top <= *hi));
        let other = prefix_ranges(&pattern_variants("1Bgh", true).unwrap());
        assert!(!other.iter().any(|(lo, hi)| *lo <= top && top <= *hi));
    }

    #[test]
    fn variants_skip_non_base58_cases() {
        let v = pattern_variants("sol", false).unwrap();
        // 'l' has no lowercase form in base58, so only 'L' remains for it.
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|s| s.ends_with('L')));
    }
}
