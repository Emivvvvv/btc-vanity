//! # Vanity Address Generator Module
//!
//! This module defines the [VanityAddr] and [SearchEngines] structs, which handle the generation
//! of vanity cryptocurrency addresses using custom patterns and regular expressions. It supports:
//! - Validation and adjustment of inputs for specific chains.
//! - Multi-threaded generation of vanity addresses.
//! - Pattern matching using prefix, suffix, anywhere, and regex modes.

#[cfg(feature = "gpu")]
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{mpsc, Arc};
#[cfg(feature = "gpu")]
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
#[cfg(feature = "gpu")]
use std::time::Instant;

use crate::error::VanityError;
use crate::vanity_addr_generator::chain::VanityChain;
use crate::vanity_addr_generator::comp::{
    contains_case_insensitive, contains_memx, eq_prefix_case_insensitive, eq_prefix_memx,
    eq_suffix_case_insensitive, eq_suffix_memx,
};
#[cfg(feature = "gpu")]
use crate::vanity_addr_generator::gpu::{
    gpu_backend_available, GpuMatch, GpuTuning, Secp256k1GpuEngine,
};
use crate::BATCH_SIZE;

#[cfg(feature = "gpu")]
use num_bigint::BigUint;
use regex::Regex;

/// Selects the execution backend for a vanity search.
///
/// `Auto` is designed for future-proofing: it lets the library choose the
/// best available backend and currently falls back to the CPU path.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum VanityBackend {
    #[default]
    Auto,
    Cpu,
    Gpu,
    Hybrid,
}

impl std::str::FromStr for VanityBackend {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            "hybrid" | "both" => Ok(Self::Hybrid),
            _ => Err(format!("Unsupported backend: {value}")),
        }
    }
}

impl std::fmt::Display for VanityBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Auto => "auto",
                Self::Cpu => "cpu",
                Self::Gpu => "gpu",
                Self::Hybrid => "hybrid",
            }
        )
    }
}

/// Bundles all search-time configuration in one place.
///
/// This gives the public API a stable extension point for future backends
/// without forcing more positional parameters into `generate`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VanitySearchOptions {
    pub threads: usize,
    pub case_sensitive: bool,
    pub fast_mode: bool,
    pub vanity_mode: VanityMode,
    pub backend: VanityBackend,
    pub gpu_batch_size: Option<usize>,
}

impl Default for VanitySearchOptions {
    fn default() -> Self {
        Self {
            threads: 8,
            case_sensitive: false,
            fast_mode: true,
            vanity_mode: VanityMode::Prefix,
            backend: VanityBackend::Auto,
            gpu_batch_size: None,
        }
    }
}

/// Identifies the elliptic-curve primitive a chain can use in the GPU backend.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GpuCurveKind {
    Secp256k1,
    Ed25519,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GpuSearchTarget {
    Bitcoin,
    Ethereum,
    Solana,
}

/// An empty struct that provides functionality for generating vanity addresses.
///
/// This struct contains only static methods and acts as a logical container for
/// vanity address generation functionality.
pub struct VanityAddr;

/// Enum to define the matching mode for vanity address generation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum VanityMode {
    /// Matches addresses that start with the pattern.
    #[default]
    Prefix,
    /// Matches addresses that end with the pattern.
    Suffix,
    /// Matches addresses that contain the pattern anywhere.
    Anywhere,
    /// Matches addresses based on a regular expression.
    Regex,
}

impl VanityAddr {
    /// Generates a vanity address for a given pattern.
    ///
    /// # Arguments
    /// - `string`: The pattern string to match against addresses.
    /// - `threads`: The number of threads to use for address generation.
    /// - `case_sensitive`: Whether the matching should be case-sensitive.
    /// - `fast_mode`: Whether to enable fast mode (with stricter limits on pattern length).
    /// - `vanity_mode`: The mode of matching (e.g., prefix, suffix).
    ///
    /// # Returns
    /// - `Ok(T)` where `T` is a type implementing [VanityChain], containing the generated address.
    /// - `Err(VanityError)` if the input is invalid or generation fails.
    ///
    /// # Behavior
    /// - Validates the input string for chain-specific rules.
    /// - Adjusts the input string based on the chain and vanity mode.
    /// - Uses multiple threads to search for a matching address.
    pub fn generate<T: VanityChain + 'static>(
        string: &str,
        threads: usize,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<T, VanityError> {
        Self::generate_with_options::<T>(
            string,
            VanitySearchOptions {
                threads,
                case_sensitive,
                fast_mode,
                vanity_mode,
                backend: VanityBackend::Auto,
                gpu_batch_size: None,
            },
        )
    }

    /// Generates a vanity address for a given pattern with explicit backend
    /// and search options.
    pub fn generate_with_options<T: VanityChain + 'static>(
        string: &str,
        options: VanitySearchOptions,
    ) -> Result<T, VanityError> {
        T::validate_input(string, options.fast_mode, options.case_sensitive)?;
        let adjusted_string = T::adjust_input(string, options.vanity_mode);

        if string.is_empty() {
            return Ok(T::generate_random());
        }

        SearchEngines::find_vanity_address::<T>(
            adjusted_string,
            options.threads,
            options.case_sensitive,
            options.vanity_mode,
            options.backend,
            options.gpu_batch_size,
        )
    }

    /// Generates a vanity address based on a regular expression.
    ///
    /// # Arguments
    /// - `regex_str`: The regular expression to match against addresses.
    /// - `threads`: The number of threads to use for address generation.
    ///
    /// # Returns
    /// - `Ok(T)` where `T` is a type implementing [VanityChain], containing the generated address.
    /// - `Err(VanityError)` if the regex is invalid or generation fails.
    ///
    /// # Behavior
    /// - Validates the regular expression for chain-specific rules.
    /// - Adjusts the regex pattern based on the chain.
    /// - Uses multiple threads to search for a matching address.
    pub fn generate_regex<T: VanityChain + 'static>(
        regex_str: &str,
        threads: usize,
    ) -> Result<T, VanityError> {
        Self::generate_regex_with_options::<T>(
            regex_str,
            VanitySearchOptions {
                threads,
                vanity_mode: VanityMode::Regex,
                backend: VanityBackend::Auto,
                ..VanitySearchOptions::default()
            },
        )
    }

    /// Generates a vanity address based on a regular expression with explicit
    /// backend and search options.
    pub fn generate_regex_with_options<T: VanityChain + 'static>(
        regex_str: &str,
        options: VanitySearchOptions,
    ) -> Result<T, VanityError> {
        T::validate_regex_pattern(regex_str)?;
        let adjusted_regex = T::adjust_regex_pattern(regex_str);

        if regex_str.is_empty() {
            return Ok(T::generate_random());
        }

        SearchEngines::find_vanity_address_regex::<T>(
            adjusted_regex,
            options.threads,
            options.backend,
        )
    }
}

/// A helper struct that implements the core logic for searching for vanity addresses.
///
/// This struct contains static methods for address search using both plain patterns
/// and regular expressions.
pub struct SearchEngines;

#[cfg(feature = "gpu")]
static AUTO_BACKEND_CACHE: OnceLock<Mutex<HashMap<String, VanityBackend>>> = OnceLock::new();
#[cfg(feature = "gpu")]
static GPU_TUNING_CACHE: OnceLock<Mutex<HashMap<String, GpuTuning>>> = OnceLock::new();
#[cfg(feature = "gpu")]
static GPU_ENGINE_CACHE: OnceLock<Mutex<Option<Arc<Secp256k1GpuEngine>>>> = OnceLock::new();
#[cfg(feature = "gpu")]
const AUTO_CPU_BATCH_FALLBACK_THRESHOLD: usize = 8_192;
#[cfg(feature = "gpu")]
const HYBRID_GPU_DOMINANT_BATCH_THRESHOLD: usize = 262_144;
#[cfg(feature = "gpu")]
const HYBRID_DEFAULT_CPU_THREAD_CAP: usize = 4;
#[cfg(feature = "gpu")]
const GPU_SHORT_PATTERN_MAX_LEN: usize = 2;
#[cfg(feature = "gpu")]
const GPU_SHORT_PATTERN_BATCH_SIZE: usize = 65_536;

#[cfg(feature = "gpu")]
struct BackendCalibration {
    cpu_attempts_per_sec: f64,
    gpu_attempts_per_sec: f64,
}

impl SearchEngines {
    #[cfg(feature = "gpu")]
    fn hybrid_trace_enabled() -> bool {
        static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
        *TRACE_ENABLED.get_or_init(|| {
            std::env::var("BTC_VANITY_HYBRID_TRACE")
                .ok()
                .map(|value| {
                    let normalized = value.trim().to_ascii_lowercase();
                    matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
                })
                .unwrap_or(false)
        })
    }

    #[cfg(feature = "gpu")]
    fn hybrid_cpu_threads(threads: usize, gpu_batch_size: Option<usize>) -> usize {
        let requested = threads.max(1);

        if let Some(override_threads) = std::env::var("BTC_VANITY_HYBRID_CPU_THREADS")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
        {
            return requested.min(override_threads);
        }

        let effective_batch = gpu_batch_size.unwrap_or(HYBRID_GPU_DOMINANT_BATCH_THRESHOLD);
        if effective_batch >= HYBRID_GPU_DOMINANT_BATCH_THRESHOLD {
            requested.min(HYBRID_DEFAULT_CPU_THREAD_CAP)
        } else {
            requested
        }
    }

    fn should_fallback_from_gpu_error(error: &VanityError) -> bool {
        matches!(
            error,
            VanityError::GpuBackendUnavailable
                | VanityError::GpuAdapterUnavailable
                | VanityError::GpuInvalidResult(_)
        )
    }

    fn matches_pattern(
        address_bytes: &[u8],
        string_bytes: &[u8],
        lower_string_bytes: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> bool {
        let pattern_len = if case_sensitive {
            string_bytes.len()
        } else {
            lower_string_bytes.len()
        };

        if address_bytes.len() < pattern_len {
            return false;
        }

        if case_sensitive {
            match vanity_mode {
                VanityMode::Prefix => eq_prefix_memx(address_bytes, string_bytes),
                VanityMode::Suffix => eq_suffix_memx(address_bytes, string_bytes),
                VanityMode::Anywhere => contains_memx(address_bytes, string_bytes),
                VanityMode::Regex => unreachable!("Regex mode should not be handled here"),
            }
        } else {
            match vanity_mode {
                VanityMode::Prefix => eq_prefix_case_insensitive(address_bytes, lower_string_bytes),
                VanityMode::Suffix => eq_suffix_case_insensitive(address_bytes, lower_string_bytes),
                VanityMode::Anywhere => {
                    contains_case_insensitive(address_bytes, lower_string_bytes)
                }
                VanityMode::Regex => unreachable!("Regex mode should not be handled here"),
            }
        }
    }

    /// Searches for a vanity address matching the given string pattern.
    ///
    /// # Arguments
    /// - `string`: The string pattern to match against addresses.
    /// - `threads`: The number of threads to use for address generation.
    /// - `case_sensitive`: Whether the matching should be case-sensitive.
    /// - `vanity_mode`: The mode of matching (e.g., prefix, suffix).
    ///
    /// # Returns
    /// - A type implementing [VanityChain] that contains the generated address.
    ///
    /// # Behavior
    /// - Spawns multiple threads to search for a matching address.
    /// - Uses an atomic flag to stop all threads once a match is found.
    /// - Uses an `mpsc` channel to send the matching address back to the main thread.
    fn find_vanity_address<T: VanityChain + 'static>(
        string: String,
        threads: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        backend: VanityBackend,
        gpu_batch_size: Option<usize>,
    ) -> Result<T, VanityError> {
        match backend {
            VanityBackend::Cpu => Ok(Self::find_vanity_address_cpu::<T>(
                string,
                threads,
                case_sensitive,
                vanity_mode,
            )),
            VanityBackend::Gpu => {
                match Self::find_vanity_address_gpu::<T>(
                    string.clone(),
                    case_sensitive,
                    vanity_mode,
                    gpu_batch_size,
                ) {
                    Ok(candidate) => Ok(candidate),
                    Err(error) if Self::should_fallback_from_gpu_error(&error) => {
                        Ok(Self::find_vanity_address_cpu::<T>(
                            string,
                            threads,
                            case_sensitive,
                            vanity_mode,
                        ))
                    }
                    Err(error) => Err(error),
                }
            }
            VanityBackend::Hybrid => {
                #[cfg(feature = "gpu")]
                {
                    Self::find_vanity_address_hybrid::<T>(
                        string,
                        threads,
                        case_sensitive,
                        vanity_mode,
                        gpu_batch_size,
                    )
                }
                #[cfg(not(feature = "gpu"))]
                {
                    Ok(Self::find_vanity_address_cpu::<T>(
                        string,
                        threads,
                        case_sensitive,
                        vanity_mode,
                    ))
                }
            }
            VanityBackend::Auto => Self::find_vanity_address_auto::<T>(
                string,
                threads,
                case_sensitive,
                vanity_mode,
                gpu_batch_size,
            ),
        }
    }

    fn find_vanity_address_auto<T: VanityChain + 'static>(
        string: String,
        threads: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
    ) -> Result<T, VanityError> {
        #[cfg(not(feature = "gpu"))]
        let _ = gpu_batch_size;

        #[cfg(feature = "gpu")]
        {
            if !matches!(vanity_mode, VanityMode::Regex) {
                if Self::should_force_cpu_for_auto(gpu_batch_size) {
                    return Ok(Self::find_vanity_address_cpu::<T>(
                        string,
                        threads,
                        case_sensitive,
                        vanity_mode,
                    ));
                }

                match Self::recommended_backend::<T>(string.len(), case_sensitive, vanity_mode) {
                    VanityBackend::Gpu => {
                        if let Ok(candidate) = Self::find_vanity_address_gpu::<T>(
                            string.clone(),
                            case_sensitive,
                            vanity_mode,
                            gpu_batch_size,
                        ) {
                            return Ok(candidate);
                        }
                    }
                    VanityBackend::Hybrid => {
                        if let Ok(candidate) = Self::find_vanity_address_hybrid::<T>(
                            string.clone(),
                            threads,
                            case_sensitive,
                            vanity_mode,
                            gpu_batch_size,
                        ) {
                            return Ok(candidate);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self::find_vanity_address_cpu::<T>(
            string,
            threads,
            case_sensitive,
            vanity_mode,
        ))
    }

    fn find_vanity_address_cpu<T: VanityChain + 'static>(
        string: String,
        threads: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> T {
        let stop = Arc::new(AtomicBool::new(false));
        loop {
            if let Some(result) = Self::find_vanity_address_cpu_cancelable::<T>(
                string.clone(),
                threads,
                case_sensitive,
                vanity_mode,
                Arc::clone(&stop),
            ) {
                return result;
            }
        }
    }

    fn find_vanity_address_cpu_cancelable<T: VanityChain + 'static>(
        string: String,
        threads: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        stop: Arc<AtomicBool>,
    ) -> Option<T> {
        let string_bytes: Arc<[u8]> = Arc::from(string.into_bytes());
        let lower_string_bytes: Arc<[u8]> = if !case_sensitive {
            Arc::from(
                string_bytes
                    .iter()
                    .map(|b| b.to_ascii_lowercase())
                    .collect::<Vec<u8>>(),
            )
        } else {
            Arc::from(Vec::<u8>::new())
        };

        let (sender, receiver) = mpsc::channel();

        for _ in 0..threads {
            let sender = sender.clone();
            let stop = Arc::clone(&stop);

            let thread_string_bytes = Arc::clone(&string_bytes);
            let thread_lower_string_bytes = Arc::clone(&lower_string_bytes);

            thread::spawn(move || {
                let mut batch: [T; BATCH_SIZE] = T::generate_batch();
                let mut dummy = T::generate_random();

                while !stop.load(Ordering::Relaxed) {
                    // Generate a batch of addresses
                    T::fill_batch(&mut batch);

                    // Check each address in the batch with loop unrolling for better performance
                    let mut i = 0;
                    while i < BATCH_SIZE {
                        // Process multiple addresses per iteration to improve cache efficiency
                        let end = std::cmp::min(i + 8, BATCH_SIZE);

                        #[allow(clippy::needless_range_loop)]
                        for j in i..end {
                            // Early exit check every few iterations to minimize atomic load overhead
                            if j.is_multiple_of(4) && stop.load(Ordering::Relaxed) {
                                return;
                            }

                            let keys_and_address = &batch[j];
                            let address_bytes = keys_and_address.get_address_bytes();
                            let matches = Self::matches_pattern(
                                address_bytes,
                                thread_string_bytes.as_ref(),
                                thread_lower_string_bytes.as_ref(),
                                case_sensitive,
                                vanity_mode,
                            );

                            // If match found...
                            if matches {
                                // Mark as found (and check if we are the first)
                                if !stop.swap(true, Ordering::Relaxed) {
                                    // We're the first thread to set found_any = true
                                    // Attempt to send the result
                                    std::mem::swap(&mut batch[j], &mut dummy);
                                    let _ = sender.send(dummy);
                                }
                                // Return immediately: no need to generate more
                                return;
                            }
                        }

                        i = end;
                    }
                }
            });
        }
        drop(sender);

        loop {
            match receiver.recv_timeout(Duration::from_millis(25)) {
                Ok(result) => return Some(result),
                Err(RecvTimeoutError::Timeout) => {
                    if stop.load(Ordering::Relaxed) {
                        return receiver.try_recv().ok();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    #[cfg(feature = "gpu")]
    fn find_vanity_address_hybrid<T: VanityChain + 'static>(
        string: String,
        threads: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
    ) -> Result<T, VanityError> {
        let fallback_string = string.clone();
        let hybrid_cpu_threads = Self::hybrid_cpu_threads(threads, gpu_batch_size);

        if matches!(vanity_mode, VanityMode::Regex) {
            return Ok(Self::find_vanity_address_cpu::<T>(
                string,
                threads,
                case_sensitive,
                vanity_mode,
            ));
        }

        if T::gpu_search_target().is_none() {
            return Ok(Self::find_vanity_address_cpu::<T>(
                string,
                threads,
                case_sensitive,
                vanity_mode,
            ));
        }

        if !gpu_backend_available() {
            return Ok(Self::find_vanity_address_cpu::<T>(
                string,
                threads,
                case_sensitive,
                vanity_mode,
            ));
        }

        let stop = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel::<Result<T, VanityError>>();

        if Self::hybrid_trace_enabled() {
            eprintln!(
                "[hybrid] starting CPU worker (threads={hybrid_cpu_threads}, requested={threads}) and GPU worker for {:?}",
                T::gpu_search_target().unwrap_or(GpuSearchTarget::Bitcoin)
            );
        }

        {
            let sender = sender.clone();
            let stop = Arc::clone(&stop);
            let string = string.clone();
            thread::spawn(move || {
                if Self::hybrid_trace_enabled() {
                    eprintln!("[hybrid] CPU worker active");
                }
                if let Some(candidate) = Self::find_vanity_address_cpu_cancelable::<T>(
                    string,
                    hybrid_cpu_threads,
                    case_sensitive,
                    vanity_mode,
                    Arc::clone(&stop),
                ) {
                    stop.store(true, Ordering::Relaxed);
                    let _ = sender.send(Ok(candidate));
                }
            });
        }

        {
            let sender = sender.clone();
            let stop = Arc::clone(&stop);
            let gpu_batch_size = gpu_batch_size;
            thread::spawn(move || {
                if Self::hybrid_trace_enabled() {
                    eprintln!("[hybrid] GPU worker active");
                }
                match Self::find_vanity_address_gpu_optimized::<T>(
                    string,
                    case_sensitive,
                    vanity_mode,
                    gpu_batch_size,
                    Some(Arc::clone(&stop)),
                ) {
                    Ok(Some(candidate)) => {
                        stop.store(true, Ordering::Relaxed);
                        let _ = sender.send(Ok(candidate));
                    }
                    Ok(None) => {}
                    Err(error) => {
                        if Self::hybrid_trace_enabled() {
                            eprintln!("[hybrid] GPU worker unavailable: {error}");
                        }
                        let _ = sender.send(Err(error));
                    }
                }
            });
        }

        drop(sender);

        let mut last_error: Option<VanityError> = None;
        while let Ok(message) = receiver.recv() {
            match message {
                Ok(candidate) => {
                    stop.store(true, Ordering::Relaxed);
                    if Self::hybrid_trace_enabled() {
                        eprintln!("[hybrid] winner selected");
                    }
                    return Ok(candidate);
                }
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }

        if let Some(error) = last_error {
            if Self::hybrid_trace_enabled() {
                eprintln!("[hybrid] falling back to CPU after worker error: {error}");
            }
        }

        Ok(Self::find_vanity_address_cpu::<T>(
            fallback_string,
            threads,
            case_sensitive,
            vanity_mode,
        ))
    }

    #[cfg(feature = "gpu")]
    fn find_vanity_address_gpu<T: VanityChain + 'static>(
        string: String,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
    ) -> Result<T, VanityError> {
        if matches!(vanity_mode, VanityMode::Regex) {
            return Err(VanityError::GpuRegexUnsupported);
        }

        match T::gpu_search_target() {
            Some(GpuSearchTarget::Bitcoin)
            | Some(GpuSearchTarget::Ethereum)
            | Some(GpuSearchTarget::Solana) => Self::find_vanity_address_gpu_optimized::<T>(
                string,
                case_sensitive,
                vanity_mode,
                gpu_batch_size,
                None,
            )
            .map(|opt| opt.unwrap()),
            _ => Err(VanityError::GpuBackendUnsupportedForChain),
        }
    }

    #[cfg(not(feature = "gpu"))]
    fn find_vanity_address_gpu<T: VanityChain + 'static>(
        _string: String,
        _case_sensitive: bool,
        _vanity_mode: VanityMode,
        _gpu_batch_size: Option<usize>,
    ) -> Result<T, VanityError> {
        Err(VanityError::GpuBackendUnavailable)
    }

    #[cfg(feature = "gpu")]
    fn find_vanity_address_gpu_optimized<T: VanityChain + 'static>(
        string: String,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
        stop: Option<Arc<AtomicBool>>,
    ) -> Result<Option<T>, VanityError> {
        let engine = Self::shared_gpu_engine()?;
        let target = T::gpu_search_target().ok_or(VanityError::GpuBackendUnsupportedForChain)?;
        let pattern_len = string.len();
        let tuning = Self::resolve_gpu_tuning::<T>(
            &engine,
            target,
            pattern_len,
            case_sensitive,
            vanity_mode,
            gpu_batch_size,
        )?;
        let seed = engine
            .generate_private_keys(1)
            .into_iter()
            .next()
            .ok_or(VanityError::GpuInvalidResult("failed to generate GPU seed"))?;

        engine
            .search_exact_with_tuning(
                target,
                seed,
                string.as_bytes(),
                case_sensitive,
                vanity_mode,
                tuning,
                stop.as_ref(),
            )?
            .map(Self::reconstruct_gpu_match::<T>)
            .transpose()
    }

    #[cfg(feature = "gpu")]
    fn shared_gpu_engine() -> Result<Arc<Secp256k1GpuEngine>, VanityError> {
        let cache = GPU_ENGINE_CACHE.get_or_init(|| Mutex::new(None));
        {
            let guard = cache
                .lock()
                .map_err(|_| VanityError::GpuInvalidResult("GPU engine cache lock poisoned"))?;
            if let Some(engine) = &*guard {
                return Ok(Arc::clone(engine));
            }
        }

        let engine = Arc::new(Secp256k1GpuEngine::new()?);
        let mut guard = cache
            .lock()
            .map_err(|_| VanityError::GpuInvalidResult("GPU engine cache lock poisoned"))?;
        if let Some(existing) = &*guard {
            return Ok(Arc::clone(existing));
        }
        *guard = Some(Arc::clone(&engine));
        Ok(engine)
    }

    #[cfg(feature = "gpu")]
    fn reconstruct_gpu_match<T: VanityChain + 'static>(
        gpu_match: GpuMatch,
    ) -> Result<T, VanityError> {
        let candidate = T::from_private_key_bytes(gpu_match.private_key_bytes)?;
        if candidate.get_address() != &gpu_match.address {
            return Err(VanityError::GpuInvalidResult(
                "GPU-reported address does not match reconstructed keypair",
            ));
        }
        Ok(candidate)
    }

    #[cfg(feature = "gpu")]
    fn recommended_backend<T: VanityChain + 'static>(
        pattern_len: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> VanityBackend {
        if matches!(vanity_mode, VanityMode::Regex) {
            return VanityBackend::Cpu;
        }

        // Keep auto mode responsive for easy searches by avoiding expensive calibration.
        if pattern_len <= GPU_SHORT_PATTERN_MAX_LEN {
            return VanityBackend::Cpu;
        }
        if pattern_len <= 4 {
            return VanityBackend::Hybrid;
        }

        let Some(target) = T::gpu_search_target() else {
            return VanityBackend::Cpu;
        };

        if !gpu_backend_available() {
            return VanityBackend::Cpu;
        }

        let Ok(engine) = Self::shared_gpu_engine() else {
            return VanityBackend::Cpu;
        };
        let cache_key =
            Self::backend_cache_key::<T>(&engine, target, pattern_len, case_sensitive, vanity_mode);
        let cache = AUTO_BACKEND_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(cached) = guard.get(&cache_key).copied() {
                return cached;
            }
        } else {
            return VanityBackend::Cpu;
        }

        let selected =
            Self::calibrate_backend::<T>(&engine, target, pattern_len, case_sensitive, vanity_mode)
                .unwrap_or(VanityBackend::Cpu);
        if let Ok(mut guard) = cache.lock() {
            guard.insert(cache_key, selected);
        }
        selected
    }

    #[cfg(not(feature = "gpu"))]
    #[allow(dead_code)]
    fn recommended_backend<T: VanityChain + 'static>(
        _pattern_len: usize,
        _case_sensitive: bool,
        _vanity_mode: VanityMode,
    ) -> VanityBackend {
        VanityBackend::Cpu
    }

    #[cfg(feature = "gpu")]
    fn calibrate_backend<T: VanityChain + 'static>(
        engine: &Secp256k1GpuEngine,
        target: GpuSearchTarget,
        pattern_len: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> Result<VanityBackend, VanityError> {
        let tuning =
            Self::get_or_calibrate_gpu_tuning::<T>(engine, target, case_sensitive, vanity_mode)?;
        if tuning.batch_size <= AUTO_CPU_BATCH_FALLBACK_THRESHOLD {
            return Ok(VanityBackend::Cpu);
        }
        let seed = engine
            .generate_private_keys(1)
            .into_iter()
            .next()
            .ok_or(VanityError::GpuInvalidResult("failed to generate GPU seed"))?;

        let pattern = match target {
            GpuSearchTarget::Bitcoin => b"zzzzzzzz".as_slice(),
            GpuSearchTarget::Ethereum => b"ffffffff".as_slice(),
            GpuSearchTarget::Solana => b"zzzzzzzz".as_slice(),
        };

        let attempts = (tuning.batch_size * tuning.ring_depth).min(16_384).max(4_096);
        let cpu_start = Instant::now();
        Self::benchmark_cpu_exact::<T>(seed, pattern, case_sensitive, vanity_mode, attempts)?;
        let cpu_elapsed = cpu_start.elapsed();

        let gpu_start = Instant::now();
        let _ = engine.run_exact_batches_with_tuning(
            target,
            seed,
            pattern,
            case_sensitive,
            vanity_mode,
            tuning,
            tuning.ring_depth,
        )?;
        let gpu_elapsed = gpu_start.elapsed();

        let calibration = BackendCalibration {
            cpu_attempts_per_sec: attempts as f64 / cpu_elapsed.as_secs_f64().max(f64::EPSILON),
            gpu_attempts_per_sec: attempts as f64 / gpu_elapsed.as_secs_f64().max(f64::EPSILON),
        };

        Ok(Self::predict_best_backend(
            calibration,
            pattern_len,
            case_sensitive,
            vanity_mode,
            tuning,
        ))
    }

    #[cfg(feature = "gpu")]
    fn estimated_attempts(pattern_len: usize, case_sensitive: bool, vanity_mode: VanityMode) -> f64 {
        // This coarse estimator is only used for backend ranking.
        let base = 58f64;
        let mut attempts = base.powi(pattern_len.max(1) as i32);

        if !case_sensitive {
            // Case-insensitive matching effectively widens acceptance for letter-heavy prefixes.
            attempts /= 2f64.powi(pattern_len as i32);
        }

        // Anywhere mode tends to match sooner than strict prefix/suffix.
        if matches!(vanity_mode, VanityMode::Anywhere) {
            attempts /= (pattern_len.max(1) as f64).min(8.0);
        }

        attempts.max(1.0)
    }

    #[cfg(feature = "gpu")]
    fn predict_best_backend(
        calibration: BackendCalibration,
        pattern_len: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        tuning: GpuTuning,
    ) -> VanityBackend {
        if pattern_len <= GPU_SHORT_PATTERN_MAX_LEN {
            return VanityBackend::Cpu;
        }

        let expected_attempts = Self::estimated_attempts(pattern_len, case_sensitive, vanity_mode);
        let cpu_secs = expected_attempts / calibration.cpu_attempts_per_sec.max(f64::EPSILON);
        let gpu_secs = expected_attempts / calibration.gpu_attempts_per_sec.max(f64::EPSILON);

        // If projected latencies are close, race CPU and GPU to optimize time-to-first-hit.
        let ratio = if cpu_secs > gpu_secs {
            cpu_secs / gpu_secs.max(f64::EPSILON)
        } else {
            gpu_secs / cpu_secs.max(f64::EPSILON)
        };
        if ratio < 1.5 {
            return VanityBackend::Hybrid;
        }

        if tuning.batch_size <= AUTO_CPU_BATCH_FALLBACK_THRESHOLD {
            VanityBackend::Cpu
        } else if gpu_secs < cpu_secs {
            VanityBackend::Gpu
        } else {
            VanityBackend::Cpu
        }
    }

    #[cfg(feature = "gpu")]
    fn get_or_calibrate_gpu_tuning<T: VanityChain + 'static>(
        engine: &Secp256k1GpuEngine,
        target: GpuSearchTarget,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> Result<GpuTuning, VanityError> {
        let cache_key = Self::tuning_cache_key(engine, target);
        let cache = GPU_TUNING_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(cached) = guard.get(&cache_key).copied() {
                return Ok(cached);
            }
        } else {
            return Err(VanityError::GpuInvalidResult(
                "GPU tuning cache lock poisoned",
            ));
        }

        let tuning = Self::calibrate_gpu_tuning::<T>(engine, target, case_sensitive, vanity_mode)?;
        if let Ok(mut guard) = cache.lock() {
            guard.insert(cache_key, tuning);
        }
        Ok(tuning)
    }

    #[cfg(feature = "gpu")]
    fn resolve_gpu_tuning<T: VanityChain + 'static>(
        engine: &Secp256k1GpuEngine,
        target: GpuSearchTarget,
        pattern_len: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
    ) -> Result<GpuTuning, VanityError> {
        if let Some(batch_size) = gpu_batch_size {
            let mut tuning = Self::cached_gpu_tuning(engine, target).unwrap_or_default();
            tuning.batch_size = batch_size;
            return Ok(tuning);
        }

        let mut tuning = Self::cached_gpu_tuning(engine, target).unwrap_or_default();

        // Short patterns have high hit probability, so reduce queue depth and batch size
        // to optimize time-to-first-hit rather than peak throughput.
        if pattern_len <= GPU_SHORT_PATTERN_MAX_LEN {
            tuning.batch_size = tuning.batch_size.min(GPU_SHORT_PATTERN_BATCH_SIZE);
            tuning.ring_depth = 1;
            tuning.candidates_per_invocation = tuning.candidates_per_invocation.min(4).max(1);
        }

        let _ = (case_sensitive, vanity_mode);
        Ok(tuning)
    }

    #[cfg(feature = "gpu")]
    fn cached_gpu_tuning(
        engine: &Secp256k1GpuEngine,
        target: GpuSearchTarget,
    ) -> Option<GpuTuning> {
        let cache_key = Self::tuning_cache_key(engine, target);
        let cache = GPU_TUNING_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            guard.get(&cache_key).copied()
        } else {
            None
        }
    }

    #[cfg(feature = "gpu")]
    fn calibrate_gpu_tuning<T: VanityChain + 'static>(
        engine: &Secp256k1GpuEngine,
        target: GpuSearchTarget,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> Result<GpuTuning, VanityError> {
        let seed = engine
            .generate_private_keys(1)
            .into_iter()
            .next()
            .ok_or(VanityError::GpuInvalidResult("failed to generate GPU seed"))?;

        let pattern = match target {
            GpuSearchTarget::Bitcoin => b"zzzzzzzz".as_slice(),
            GpuSearchTarget::Ethereum => b"ffffffff".as_slice(),
            GpuSearchTarget::Solana => b"zzzzzzzz".as_slice(),
        };

        let mut best = GpuTuning::default();
        let mut best_throughput = 0.0f64;
        let mut no_gain_streak = 0usize;

        let mut batch_candidates = Vec::new();
        // M1 Pro calibration showed <= 8,192 has worse latency/throughput than CPU.
        let mut current = 8_192usize;
        while current < crate::vanity_addr_generator::gpu::GPU_MAX_BATCH_SIZE {
            batch_candidates.push(current);
            current = current.saturating_mul(2);
        }
        batch_candidates.push(crate::vanity_addr_generator::gpu::GPU_MAX_BATCH_SIZE);

        for batch_size in batch_candidates {
            let mut best_for_batch: Option<(GpuTuning, f64)> = None;

            for candidates_per_invocation in [1u32, 2, 4, 8, 16] {
                for ring_depth in [2usize, 3, 4] {
                    let tuning = GpuTuning {
                        batch_size,
                        workgroup_size: GpuTuning::default().workgroup_size,
                        candidates_per_invocation,
                        ring_depth,
                    };
                    let start = Instant::now();
                    let _ = engine.run_exact_batches_with_tuning(
                        target,
                        seed,
                        pattern,
                        case_sensitive,
                        vanity_mode,
                        tuning,
                        tuning.ring_depth,
                    )?;
                    let elapsed = start.elapsed();
                    let attempts = (tuning.batch_size * tuning.ring_depth) as f64;
                    let throughput = attempts / elapsed.as_secs_f64().max(f64::EPSILON);

                    if best_for_batch
                        .map(|(_, current_throughput)| throughput > current_throughput)
                        .unwrap_or(true)
                    {
                        best_for_batch = Some((tuning, throughput));
                    }
                }
            }

            if let Some((batch_tuning, batch_throughput)) = best_for_batch {
                if batch_throughput > best_throughput * 1.03 {
                    best = batch_tuning;
                    best_throughput = batch_throughput;
                    no_gain_streak = 0;
                } else {
                    no_gain_streak += 1;
                }

                if batch_size >= 262_144 && no_gain_streak >= 2 {
                    break;
                }
            }
        }

        Ok(best)
    }

    #[cfg(feature = "gpu")]
    fn tuning_cache_key(engine: &Secp256k1GpuEngine, target: GpuSearchTarget) -> String {
        format!("{}:{target:?}", engine.adapter_name())
    }

    #[cfg(feature = "gpu")]
    fn backend_cache_key<T: VanityChain + 'static>(
        engine: &Secp256k1GpuEngine,
        target: GpuSearchTarget,
        pattern_len: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> String {
        let pattern_bucket = if pattern_len <= 2 {
            "l2"
        } else if pattern_len <= 4 {
            "l4"
        } else if pattern_len <= 6 {
            "l6"
        } else {
            "l7plus"
        };
        format!(
            "{}:{}:{target:?}:{pattern_bucket}:{vanity_mode:?}:{case_sensitive}",
            engine.adapter_name(),
            std::any::type_name::<T>(),
        )
    }

    #[cfg(feature = "gpu")]
    fn should_force_cpu_for_auto(gpu_batch_size: Option<usize>) -> bool {
        gpu_batch_size
            .map(|batch_size| batch_size <= AUTO_CPU_BATCH_FALLBACK_THRESHOLD)
            .unwrap_or(false)
    }

    #[cfg(feature = "gpu")]
    fn benchmark_cpu_exact<T: VanityChain + 'static>(
        seed: [u8; 32],
        string_bytes: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        attempts: usize,
    ) -> Result<(), VanityError> {
        let lower_string_bytes = if case_sensitive {
            Vec::new()
        } else {
            string_bytes
                .iter()
                .map(|byte| byte.to_ascii_lowercase())
                .collect::<Vec<u8>>()
        };

        for offset in 0..attempts {
            let private_key = Self::secp256k1_scalar_from_seed(seed, offset as u64);
            let candidate = T::from_private_key_bytes(private_key)?;
            let _ = Self::matches_pattern(
                candidate.get_address_bytes(),
                string_bytes,
                &lower_string_bytes,
                case_sensitive,
                vanity_mode,
            );
        }

        Ok(())
    }

    #[cfg(feature = "gpu")]
    fn secp256k1_scalar_from_seed(seed: [u8; 32], offset: u64) -> [u8; 32] {
        const SECP256K1_ORDER_BE: [u8; 32] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFE, 0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B, 0xBF, 0xD2, 0x5E, 0x8C,
            0xD0, 0x36, 0x41, 0x41,
        ];
        let order = BigUint::from_bytes_be(&SECP256K1_ORDER_BE);
        let seed_big = BigUint::from_bytes_be(&seed);
        let mut scalar = (seed_big + BigUint::from(offset)) % &order;
        if scalar == BigUint::default() {
            scalar = BigUint::from(1u8);
        }

        let bytes = scalar.to_bytes_be();
        let mut result = [0u8; 32];
        let start = 32 - bytes.len();
        result[start..].copy_from_slice(&bytes);
        result
    }

    /// Searches for a vanity address matching the given regex pattern.
    ///
    /// # Arguments
    /// - `regex_str`: The regex pattern to match against addresses.
    /// - `threads`: The number of threads to use for address generation.
    ///
    /// # Returns
    /// - `Ok(T)` where `T` is a type implementing [VanityChain], containing the generated address.
    /// - `Err(VanityError)` if the regex is invalid or generation fails.
    ///
    /// # Behavior
    /// - Spawns multiple threads to search for a matching address.
    /// - Uses an atomic flag to stop all threads once a match is found.
    /// - Uses an `mpsc` channel to send the matching address back to the main thread.
    pub fn find_vanity_address_regex<T: VanityChain + 'static>(
        regex_str: String,
        threads: usize,
        backend: VanityBackend,
    ) -> Result<T, VanityError> {
        match backend {
            VanityBackend::Cpu => Self::find_vanity_address_regex_cpu::<T>(regex_str, threads),
            VanityBackend::Gpu => Err(VanityError::GpuRegexUnsupported),
            VanityBackend::Hybrid => Self::find_vanity_address_regex_cpu::<T>(regex_str, threads),
            VanityBackend::Auto => Self::find_vanity_address_regex_cpu::<T>(regex_str, threads),
        }
    }

    fn find_vanity_address_regex_cpu<T: VanityChain + 'static>(
        regex_str: String,
        threads: usize,
    ) -> Result<T, VanityError> {
        // Validate the regex syntax
        let _test_regex = Regex::new(&regex_str).map_err(|_e| VanityError::InvalidRegex)?;

        let (sender, receiver) = mpsc::channel();
        let found_any = Arc::new(AtomicBool::new(false));

        for _ in 0..threads {
            let sender = sender.clone();
            let found_any = Arc::clone(&found_any);
            let regex = Regex::new(&regex_str).map_err(|_e| VanityError::InvalidRegex)?;

            thread::spawn(move || {
                let mut batch: [T; BATCH_SIZE] = T::generate_batch();
                let mut dummy = T::generate_random();

                while !found_any.load(Ordering::Relaxed) {
                    // Generate a batch of addresses
                    T::fill_batch(&mut batch);

                    // Check each address in the batch
                    for (i, keys_and_address) in batch.iter().enumerate() {
                        let address = keys_and_address.get_address();
                        if regex.is_match(address) && !found_any.load(Ordering::Relaxed) {
                            // If a match is found, send it to the main thread
                            if !found_any.swap(true, Ordering::Relaxed) {
                                std::mem::swap(&mut batch[i], &mut dummy);
                                let _ = sender.send(dummy);
                                return;
                            }
                        }
                    }
                }
            });
        }

        // The main thread just waits for the first successful result.
        // As soon as one thread sends over the channel, we have our vanity address.
        receiver.recv().map_err(|_| {
            VanityError::VanityGeneratorError(
                "regex workers exited before finding a matching address",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{VanityAddr, VanityBackend, VanityMode};

    #[test]
    fn test_parse_vanity_backend() {
        assert_eq!(
            "auto".parse::<VanityBackend>().unwrap(),
            VanityBackend::Auto
        );
        assert_eq!("cpu".parse::<VanityBackend>().unwrap(), VanityBackend::Cpu);
        assert_eq!("gpu".parse::<VanityBackend>().unwrap(), VanityBackend::Gpu);
        assert_eq!(
            "hybrid".parse::<VanityBackend>().unwrap(),
            VanityBackend::Hybrid
        );
        assert_eq!(
            "both".parse::<VanityBackend>().unwrap(),
            VanityBackend::Hybrid
        );
        assert!("metal".parse::<VanityBackend>().is_err());
    }

    mod bitcoin_vanity_tests {
        use super::*;
        use crate::keys_and_address::BitcoinKeyPair;

        #[cfg(feature = "gpu")]
        #[test]
        fn test_auto_backend_forces_cpu_when_batch_override_is_small() {
            assert!(super::super::SearchEngines::should_force_cpu_for_auto(
                Some(8_192)
            ));
            assert!(super::super::SearchEngines::should_force_cpu_for_auto(
                Some(4_096)
            ));
            assert!(!super::super::SearchEngines::should_force_cpu_for_auto(
                Some(16_384)
            ));
            assert!(!super::super::SearchEngines::should_force_cpu_for_auto(
                None
            ));
        }

        #[test]
        fn test_generate_vanity_prefix() {
            let vanity_string = "et";
            let keys_and_address = VanityAddr::generate::<BitcoinKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                true,               // Case-insensitivity
                true,               // Fast mode (limits string size with 4 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();

            let vanity_addr_starts_with = "1et";
            assert!(keys_and_address
                .get_comp_address()
                .starts_with(vanity_addr_starts_with));
        }

        #[test]
        fn test_generate_vanity_suffix() {
            let vanity_string = "12";
            let keys_and_address = VanityAddr::generate::<BitcoinKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 4 characters)
                VanityMode::Suffix, // Vanity mode set to Suffix
            )
            .unwrap();

            assert!(keys_and_address.get_comp_address().ends_with(vanity_string));
        }

        #[test]
        fn test_generate_vanity_anywhere() {
            let vanity_string = "ab";
            let keys_and_address = VanityAddr::generate::<BitcoinKeyPair>(
                vanity_string,
                4,                    // Use 4 threads
                true,                 // Case-insensitivity
                true,                 // Fast mode (limits string size with 4 characters)
                VanityMode::Anywhere, // Vanity mode set to Anywhere
            )
            .unwrap();

            assert!(keys_and_address.get_comp_address().contains(vanity_string));
        }

        #[test]
        #[should_panic(expected = "FastModeEnabled")]
        fn test_generate_vanity_string_too_long_with_fast_mode() {
            let vanity_string = "123456"; // String longer than 5 characters
            let _ = VanityAddr::generate::<BitcoinKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 4 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase58")]
        fn test_generate_vanity_invalid_base58() {
            let vanity_string = "emiO"; // Contains invalid base58 character 'O'
            let _ = VanityAddr::generate::<BitcoinKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 4 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        fn test_generate_regex_et_ends() {
            let pattern = "ET$";
            let keys_and_address = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4)
                .expect("Failed to generate address for 'ET$'");
            let address = keys_and_address.get_comp_address();

            // The final pattern is "ET$" => ends with "ET"
            assert!(
                address.ends_with("ET"),
                "Address should end with 'ET': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_rewrite() {
            // Original pattern is '^E' (not '^1'), so the code will insert '1', resulting in '^1E'.
            // We expect it eventually to find an address starting with "1E".
            let pattern = "^E";
            let keys_and_address =
                VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_comp_address();
            // Now that we know it's '^1E', check the first two characters:
            assert!(
                address.starts_with("1E"),
                "Address should start with '1E': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_e_any_t() {
            // Must start with "1E" (rewritten from "^E") and end with "T".
            let pattern = "^E.*T$";
            let keys_and_address = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4)
                .expect("Failed to generate address for '^E.*T$'");
            let address = keys_and_address.get_comp_address();

            // Because of rewriting, the actual pattern used is '^1E.*T$'.
            // 1) Check it starts with "1E"
            assert!(
                address.starts_with("1E"),
                "Address should start with '1E': {}",
                address
            );
            // 2) Check it ends with 'T'
            assert!(
                address.ends_with('T'),
                "Address should end with 'T': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_e_69_any_t() {
            // Must start with "1E", contain "69", and end with "T".
            // Rewritten from "^E.*69.*T$" => "^1E.*69.*T$"
            let pattern = "^E.*69.*T$";
            let keys_and_address = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4)
                .expect("Failed to generate address for '^E.*69.*T$'");
            let address = keys_and_address.get_comp_address();

            // After rewriting: '^1E.*69.*T$'
            assert!(
                address.starts_with("1E"),
                "Address should start with '1E': {}",
                address
            );
            assert!(
                address.contains("69"),
                "Address should contain '69': {}",
                address
            );
            assert!(
                address.ends_with('T'),
                "Address should end with 'T': {}",
                address
            );
        }

        #[test]
        #[should_panic(expected = "InvalidRegex")]
        fn test_generate_regex_invalid_syntax() {
            let pattern = "^(abc";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_forbidden_char_zero() {
            let pattern = "^0";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_forbidden_char_o() {
            let pattern = "^O";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_forbidden_char_i() {
            let pattern = "^I";
            let _ = VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, 4).unwrap();
        }

        #[cfg(not(feature = "gpu"))]
        #[test]
        fn test_generate_with_gpu_backend_is_unavailable() {
            let result = VanityAddr::generate_with_options::<BitcoinKeyPair>(
                "et",
                super::super::VanitySearchOptions {
                    threads: 1,
                    case_sensitive: true,
                    fast_mode: true,
                    vanity_mode: VanityMode::Prefix,
                    backend: VanityBackend::Gpu,
                    gpu_batch_size: None,
                },
            );

            assert!(matches!(
                result,
                Err(crate::error::VanityError::GpuBackendUnavailable)
            ));
        }
    }

    #[cfg(feature = "ethereum")]
    mod ethereum_vanity_tests {
        use super::*;
        use crate::keys_and_address::{EthereumKeyPair, KeyPairGenerator};

        #[test]
        fn test_generate_vanity_prefix() {
            let vanity_string = "ab";
            let keys_and_address = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();

            let expected_prefix = "ab";
            assert!(keys_and_address
                .get_address()
                .to_lowercase()
                .starts_with(expected_prefix));
        }

        #[test]
        fn test_generate_vanity_suffix() {
            let vanity_string = "123";
            let keys_and_address = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode
                VanityMode::Suffix, // Vanity mode set to Suffix
            )
            .unwrap();

            assert!(keys_and_address.get_address().ends_with(vanity_string));
        }

        #[test]
        fn test_generate_vanity_anywhere() {
            let vanity_string = "abc";
            let keys_and_address = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                    // Use 4 threads
                false,                // Case-insensitivity
                true,                 // Fast mode (limits string size to 16 characters)
                VanityMode::Anywhere, // Vanity mode set to Anywhere
            )
            .unwrap();

            assert!(keys_and_address.get_address().contains(vanity_string));
        }

        #[test]
        #[should_panic(expected = "FastModeEnabled")]
        fn test_generate_vanity_string_too_long_with_fast_mode() {
            let vanity_string = "12345678901234567890"; // String longer than 16 characters
            let _ = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode (limits string size to 16 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase16")]
        fn test_generate_vanity_invalid_base16() {
            let vanity_string = "g123"; // Contains invalid base16 character 'g'
            let _ = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase16")]
        fn test_generate_vanity_with_prefix() {
            let vanity_string = "0xdead"; // Contains invalid base16 character 'x'
            let _ = VanityAddr::generate::<EthereumKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-sensitivity
                true,               // Fast mode
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        fn test_generate_regex_prefix() {
            let pattern = "^ab";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("ab"),
                "Address should start with 'ab': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_suffix() {
            let pattern = "cd$";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("cd"),
                "Address should end with 'cd': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_anywhere() {
            let pattern = ".*abc.*";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("abc"),
                "Address should contain 'abc': {}",
                address
            );
        }

        #[test]
        #[should_panic(expected = "InvalidRegex")]
        fn test_generate_regex_invalid_syntax() {
            let pattern = "^(abc";
            let _ = VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase16")]
        fn test_generate_regex_invalid_characters() {
            let pattern = "^gh";
            let _ = VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase16")]
        fn test_generate_regex_with_prefix() {
            let pattern = "^0xdead";
            let _ = VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        fn test_generate_regex_complex_pattern() {
            let pattern = "^ab.*12$";
            let keys_and_address =
                VanityAddr::generate_regex::<EthereumKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("ab"),
                "Address should start with 'ab': {}",
                address
            );
            assert!(
                address.ends_with("12"),
                "Address should end with '12': {}",
                address
            );
        }
    }

    #[cfg(feature = "solana")]
    mod solana_vanity_tests {
        use super::*;
        use crate::keys_and_address::{KeyPairGenerator, SolanaKeyPair};

        #[test]
        fn test_generate_vanity_prefix() {
            let vanity_string = "et";
            let keys_and_address = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                true,               // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();

            let vanity_addr_starts_with = "et";
            assert!(keys_and_address
                .get_address()
                .starts_with(vanity_addr_starts_with));
        }

        #[test]
        fn test_generate_vanity_suffix() {
            let vanity_string = "12";
            let keys_and_address = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Suffix, // Vanity mode set to Suffix
            )
            .unwrap();

            assert!(keys_and_address.get_address().ends_with(vanity_string));
        }

        #[test]
        fn test_generate_vanity_anywhere() {
            let vanity_string = "ab";
            let keys_and_address = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                    // Use 4 threads
                true,                 // Case-insensitivity
                true,                 // Fast mode (limits string size with 44 characters)
                VanityMode::Anywhere, // Vanity mode set to Anywhere
            )
            .unwrap();

            assert!(keys_and_address.get_address().contains(vanity_string));
        }

        #[test]
        #[should_panic(expected = "FastModeEnabled")]
        fn test_generate_vanity_string_too_long_with_fast_mode() {
            let vanity_string = "123456"; // String longer than 5 characters
            let _ = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        #[should_panic(expected = "InputNotBase58")]
        fn test_generate_vanity_invalid_base58() {
            let vanity_string = "emiO"; // Contains invalid base58 character 'O'
            let _ = VanityAddr::generate::<SolanaKeyPair>(
                vanity_string,
                4,                  // Use 4 threads
                false,              // Case-insensitivity
                true,               // Fast mode (limits string size with 44 characters)
                VanityMode::Prefix, // Vanity mode set to Prefix
            )
            .unwrap();
        }

        #[test]
        fn test_generate_regex_prefix() {
            let pattern = "^et";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("et"),
                "Address should start with 'et': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_suffix() {
            let pattern = "cd$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("cd"),
                "Address should end with 'cd': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_anywhere() {
            let pattern = ".*ab.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("ab"),
                "Address should contain 'ab': {}",
                address
            );
        }

        #[test]
        #[should_panic(expected = "InvalidRegex")]
        fn test_generate_regex_invalid_syntax() {
            let pattern = "^(abc";
            let _ = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        #[should_panic(expected = "RegexNotBase58")]
        fn test_generate_regex_invalid_characters() {
            let pattern = "^ghO";
            let _ = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
        }

        #[test]
        fn test_generate_regex_starts_with_e() {
            let pattern = "^e";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("e"),
                "Address should start with 'e': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_contains_11() {
            let pattern = ".*11.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("11"),
                "Address should contain '11': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_contains_22() {
            let pattern = ".*22.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("22"),
                "Address should contain '22': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_ends_with_t() {
            let pattern = "t$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("t"),
                "Address should end with 't': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_complex_sequence() {
            let pattern = "11.*22";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("11") && address.contains("22"),
                "Address should contain '11' followed by '22': {}",
                address
            );
        }

        #[test]
        fn test_generate_regex_complex_pattern() {
            let pattern = "^e.*9.*t$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("e"),
                "Address should start with 'e': {}",
                address
            );
            assert!(
                address.contains("9"),
                "Address should contain '9': {}",
                address
            );
            assert!(
                address.ends_with("t"),
                "Address should end with 't': {}",
                address
            );
        }
    }
}
