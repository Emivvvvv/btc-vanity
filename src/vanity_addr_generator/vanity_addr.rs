//! # Vanity Address Generator Module
//!
//! This module defines the [VanityAddr] and [SearchEngines] structs, which handle the generation
//! of vanity cryptocurrency addresses using custom patterns and regular expressions. It supports:
//! - Validation and adjustment of inputs for specific chains.
//! - Multi-threaded generation of vanity addresses.
//! - Pattern matching using prefix, suffix, anywhere, and regex modes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{mpsc, Arc};
#[cfg(feature = "gpu")]
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::error::VanityError;
use crate::vanity_addr_generator::chain::VanityChain;
use crate::vanity_addr_generator::comp::CompiledPattern;
#[cfg(feature = "gpu")]
use crate::vanity_addr_generator::gpu::{GpuMatch, GpuTuning, Secp256k1GpuEngine};
use crate::BATCH_SIZE;

use regex::Regex;

pub(crate) fn default_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

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
    /// Optional best-effort GPU dispatch duty cycle from 1 to 100 percent.
    pub gpu_usage_limit: Option<u8>,
}

impl Default for VanitySearchOptions {
    fn default() -> Self {
        Self {
            threads: default_thread_count(),
            case_sensitive: false,
            fast_mode: true,
            vanity_mode: VanityMode::Prefix,
            backend: VanityBackend::Auto,
            gpu_batch_size: None,
            gpu_usage_limit: None,
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
                gpu_usage_limit: None,
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
        if options.threads == 0 {
            return Err(VanityError::InvalidThreadCount);
        }
        if let Some(limit) = options.gpu_usage_limit {
            if !(1..=100).contains(&limit) {
                return Err(VanityError::InvalidGpuUsageLimit);
            }
        }

        SearchEngines::find_vanity_address::<T>(
            adjusted_string,
            options.threads,
            options.case_sensitive,
            options.vanity_mode,
            options.backend,
            options.gpu_batch_size,
            options.gpu_usage_limit,
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
        if options.threads == 0 {
            return Err(VanityError::InvalidThreadCount);
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
static GPU_ENGINE_CACHE: OnceLock<Mutex<Option<Arc<Secp256k1GpuEngine>>>> = OnceLock::new();
#[cfg(feature = "gpu")]
const HYBRID_GPU_DOMINANT_BATCH_THRESHOLD: usize = 262_144;
#[cfg(feature = "gpu")]
const HYBRID_DEFAULT_CPU_THREAD_CAP: usize = 4;
#[cfg(feature = "gpu")]
const GPU_SHORT_PATTERN_MAX_LEN: usize = 2;
#[cfg(feature = "gpu")]
const GPU_SHORT_PATTERN_BATCH_SIZE: usize = 65_536;

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
        gpu_usage_limit: Option<u8>,
    ) -> Result<T, VanityError> {
        match backend {
            VanityBackend::Cpu => Ok(Self::find_vanity_address_cpu::<T>(
                string,
                threads,
                case_sensitive,
                vanity_mode,
            )),
            VanityBackend::Gpu => Self::find_vanity_address_gpu::<T>(
                string,
                case_sensitive,
                vanity_mode,
                gpu_batch_size,
                gpu_usage_limit,
            ),
            VanityBackend::Hybrid => {
                #[cfg(feature = "gpu")]
                {
                    Self::find_vanity_address_hybrid::<T>(
                        string,
                        threads,
                        case_sensitive,
                        vanity_mode,
                        gpu_batch_size,
                        gpu_usage_limit,
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
                gpu_usage_limit,
            ),
        }
    }

    fn find_vanity_address_auto<T: VanityChain + 'static>(
        string: String,
        threads: usize,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
        gpu_usage_limit: Option<u8>,
    ) -> Result<T, VanityError> {
        #[cfg(not(feature = "gpu"))]
        let _ = (gpu_batch_size, gpu_usage_limit);

        #[cfg(feature = "gpu")]
        {
            if !matches!(vanity_mode, VanityMode::Regex) {
                match Self::recommended_backend::<T>(string.len(), vanity_mode) {
                    VanityBackend::Gpu => {
                        if let Ok(candidate) = Self::find_vanity_address_gpu::<T>(
                            string.clone(),
                            case_sensitive,
                            vanity_mode,
                            gpu_batch_size,
                            gpu_usage_limit,
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
                            gpu_usage_limit,
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
        let matcher = Arc::new(CompiledPattern::new(
            string.as_bytes(),
            case_sensitive,
            vanity_mode,
        ));

        let (sender, receiver) = mpsc::channel();

        for _ in 0..threads {
            let sender = sender.clone();
            let stop = Arc::clone(&stop);

            let matcher = Arc::clone(&matcher);

            thread::spawn(move || {
                let mut batch: [T; BATCH_SIZE] = T::generate_batch();
                let mut dummy = T::generate_random();

                while !stop.load(Ordering::Relaxed) {
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
                            let matches = matcher.matches(address_bytes);

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

                    if !stop.load(Ordering::Relaxed) {
                        T::fill_batch(&mut batch);
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
        gpu_usage_limit: Option<u8>,
    ) -> Result<T, VanityError> {
        let fallback_string = string.clone();
        let hybrid_cpu_threads = Self::hybrid_cpu_threads(threads, gpu_batch_size);
        let gpu_usage_limit =
            Self::effective_gpu_usage_limit(gpu_usage_limit, VanityBackend::Hybrid);

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
            thread::spawn(move || {
                if Self::hybrid_trace_enabled() {
                    eprintln!("[hybrid] GPU worker active");
                }
                match Self::find_vanity_address_gpu_optimized::<T>(
                    string,
                    case_sensitive,
                    vanity_mode,
                    gpu_batch_size,
                    gpu_usage_limit,
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
        gpu_usage_limit: Option<u8>,
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
                Self::effective_gpu_usage_limit(gpu_usage_limit, VanityBackend::Gpu),
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
        _gpu_usage_limit: Option<u8>,
    ) -> Result<T, VanityError> {
        Err(VanityError::GpuBackendUnavailable)
    }

    #[cfg(feature = "gpu")]
    fn find_vanity_address_gpu_optimized<T: VanityChain + 'static>(
        string: String,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        gpu_batch_size: Option<usize>,
        gpu_usage_limit: u8,
        stop: Option<Arc<AtomicBool>>,
    ) -> Result<Option<T>, VanityError> {
        let engine = Self::shared_gpu_engine()?;
        let target = T::gpu_search_target().ok_or(VanityError::GpuBackendUnsupportedForChain)?;
        let pattern_len = string.len();
        let tuning = Self::resolve_gpu_tuning(pattern_len, gpu_batch_size)?;
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
                gpu_usage_limit,
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
        vanity_mode: VanityMode,
    ) -> VanityBackend {
        let policy_with_gpu = Self::auto_backend_policy(pattern_len, vanity_mode, true);
        if matches!(policy_with_gpu, VanityBackend::Cpu) {
            return VanityBackend::Cpu;
        }

        let gpu_available = T::gpu_search_target().is_some() && Self::shared_gpu_engine().is_ok();
        Self::auto_backend_policy(pattern_len, vanity_mode, gpu_available)
    }

    #[cfg(feature = "gpu")]
    fn auto_backend_policy(
        pattern_len: usize,
        vanity_mode: VanityMode,
        gpu_available: bool,
    ) -> VanityBackend {
        if !gpu_available
            || matches!(vanity_mode, VanityMode::Regex)
            || pattern_len <= GPU_SHORT_PATTERN_MAX_LEN
        {
            return VanityBackend::Cpu;
        }

        if pattern_len <= 4 {
            VanityBackend::Hybrid
        } else {
            VanityBackend::Gpu
        }
    }

    #[cfg(feature = "gpu")]
    fn effective_gpu_usage_limit(requested: Option<u8>, backend: VanityBackend) -> u8 {
        requested.unwrap_or({
            if matches!(backend, VanityBackend::Hybrid) {
                70
            } else {
                100
            }
        })
    }

    #[cfg(feature = "gpu")]
    fn resolve_gpu_tuning(
        pattern_len: usize,
        gpu_batch_size: Option<usize>,
    ) -> Result<GpuTuning, VanityError> {
        let mut tuning = GpuTuning::default();
        if let Some(batch_size) = gpu_batch_size {
            if batch_size == 0 {
                return Err(VanityError::GpuInvalidResult("invalid GPU batch size"));
            }
            tuning.batch_size =
                batch_size.min(crate::vanity_addr_generator::gpu::GPU_MAX_BATCH_SIZE);
        }

        // Short patterns have high hit probability, so reduce queue depth and batch size
        // to optimize time-to-first-hit rather than peak throughput.
        if pattern_len <= GPU_SHORT_PATTERN_MAX_LEN {
            tuning.batch_size = tuning.batch_size.min(GPU_SHORT_PATTERN_BATCH_SIZE);
            tuning.ring_depth = 1;
            tuning.candidates_per_invocation = tuning.candidates_per_invocation.clamp(1, 4);
        }

        Ok(tuning)
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

                    if !found_any.load(Ordering::Relaxed) {
                        T::fill_batch(&mut batch);
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
    #[cfg(feature = "gpu")]
    use super::SearchEngines;
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

    #[cfg(feature = "gpu")]
    #[test]
    fn test_auto_backend_policy_is_deterministic() {
        assert_eq!(
            SearchEngines::auto_backend_policy(8, VanityMode::Regex, true),
            VanityBackend::Cpu
        );
        assert_eq!(
            SearchEngines::auto_backend_policy(8, VanityMode::Prefix, false),
            VanityBackend::Cpu
        );

        for pattern_len in 0..=2 {
            assert_eq!(
                SearchEngines::auto_backend_policy(pattern_len, VanityMode::Prefix, true),
                VanityBackend::Cpu
            );
        }
        for pattern_len in 3..=4 {
            assert_eq!(
                SearchEngines::auto_backend_policy(pattern_len, VanityMode::Prefix, true),
                VanityBackend::Hybrid
            );
        }
        assert_eq!(
            SearchEngines::auto_backend_policy(5, VanityMode::Prefix, true),
            VanityBackend::Gpu
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn test_hybrid_defaults_to_reduced_gpu_usage() {
        assert_eq!(
            SearchEngines::effective_gpu_usage_limit(None, VanityBackend::Hybrid),
            70
        );
        assert_eq!(
            SearchEngines::effective_gpu_usage_limit(None, VanityBackend::Gpu),
            100
        );
        assert_eq!(
            SearchEngines::effective_gpu_usage_limit(Some(100), VanityBackend::Hybrid),
            100
        );
    }

    mod bitcoin_vanity_tests {
        use super::*;
        use crate::keys_and_address::BitcoinKeyPair;

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
                "Address should end with 'ET': {address}"
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
                "Address should start with '1E': {address}"
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
                "Address should start with '1E': {address}"
            );
            // 2) Check it ends with 'T'
            assert!(
                address.ends_with('T'),
                "Address should end with 'T': {address}"
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
                "Address should start with '1E': {address}"
            );
            assert!(
                address.contains("69"),
                "Address should contain '69': {address}"
            );
            assert!(
                address.ends_with('T'),
                "Address should end with 'T': {address}"
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
                    gpu_usage_limit: Some(100),
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
                "Address should start with 'ab': {address}"
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
                "Address should end with 'cd': {address}"
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
                "Address should contain 'abc': {address}"
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
                "Address should start with 'ab': {address}"
            );
            assert!(
                address.ends_with("12"),
                "Address should end with '12': {address}"
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
                "Address should start with 'et': {address}"
            );
        }

        #[test]
        fn test_generate_regex_suffix() {
            let pattern = "cd$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("cd"),
                "Address should end with 'cd': {address}"
            );
        }

        #[test]
        fn test_generate_regex_anywhere() {
            let pattern = ".*ab.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("ab"),
                "Address should contain 'ab': {address}"
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
                "Address should start with 'e': {address}"
            );
        }

        #[test]
        fn test_generate_regex_contains_11() {
            let pattern = ".*11.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("11"),
                "Address should contain '11': {address}"
            );
        }

        #[test]
        fn test_generate_regex_contains_22() {
            let pattern = ".*22.*";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("22"),
                "Address should contain '22': {address}"
            );
        }

        #[test]
        fn test_generate_regex_ends_with_t() {
            let pattern = "t$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.ends_with("t"),
                "Address should end with 't': {address}"
            );
        }

        #[test]
        fn test_generate_regex_complex_sequence() {
            let pattern = "11.*22";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.contains("11") && address.contains("22"),
                "Address should contain '11' followed by '22': {address}"
            );
        }

        #[test]
        fn test_generate_regex_complex_pattern() {
            let pattern = "^e.*9.*t$";
            let keys_and_address = VanityAddr::generate_regex::<SolanaKeyPair>(pattern, 4).unwrap();
            let address = keys_and_address.get_address();

            assert!(
                address.starts_with("e"),
                "Address should start with 'e': {address}"
            );
            assert!(
                address.contains("9"),
                "Address should contain '9': {address}"
            );
            assert!(
                address.ends_with("t"),
                "Address should end with 't': {address}"
            );
        }
    }
}
