//! # CLI and Input File Flags Module
//!
//! This module handles the extraction and management of flags and configuration options
//! from the command-line interface (CLI) and input files. It provides mechanisms to:
//! - Parse flags and input patterns from the CLI.
//! - Combine and prioritize CLI-level and file-based flags.

use crate::vanity_addr_generator::chain::Chain;
use crate::{VanityBackend, VanityMode};
use clap::ArgMatches;

/// Represents the configuration flags for vanity address generation.
#[derive(Debug, Clone, Default)]
pub struct VanityFlags {
    /// The number of threads to use for generation.
    pub threads: usize,
    /// The name of the output file, if specified.
    pub output_file_name: Option<String>,

    /// If `true`, CLI flags override file-based flags.
    pub force_flags: bool,
    /// If `true`, pattern matching is case-sensitive.
    pub is_case_sensitive: bool,
    /// If `true`, disables fast mode. Fast mode puts a length limit for the searching string.
    pub disable_fast_mode: bool,
    /// Specifies the mode of matching (e.g., `prefix`, `suffix`, `anywhere`, `regex`).
    pub vanity_mode: Option<VanityMode>,
    /// Specifies the blockchain type (e.g., `Bitcoin`, `Ethereum`, `Solana`).
    pub chain: Option<Chain>,
    /// Selects the execution backend.
    pub backend: Option<VanityBackend>,
    /// Optional fixed GPU batch size override.
    pub gpu_batch_size: Option<usize>,
}

/// Enum representing the source of the vanity patterns.
#[derive(Debug)]
pub enum PatternsSource {
    /// A single pattern provided directly via the CLI.
    SingleString(String),
    /// Patterns read from an input file specified by its path.
    InputFile(String),
}

/// Parses CLI arguments into [VanityFlags] and determines the source of the vanity patterns.
///
/// # Arguments
/// - `matches`: The `ArgMatches` object provided by the `clap` library.
///
/// # Returns
/// - A tuple containing:
///   - `VanityFlags`: The parsed configuration flags.
///   - `PatternsSource`: The source of the vanity patterns (either a single string or an input file).
///
/// # Behavior
/// - Determines the blockchain (`chain`) based on flags (e.g., `ethereum`, `solana`, `bitcoin`).
/// - Determines the vanity mode (`vanity_mode`) based on flags (e.g., `regex`, `anywhere`, `suffix`, `prefix`).
/// - Parses the number of threads, defaulting to 8 if not specified.
/// - Detects whether patterns are provided via a single string or an input file.
pub fn parse_cli(matches: ArgMatches) -> (VanityFlags, PatternsSource) {
    // 1) Extract chain
    let chain = if matches.get_flag("ethereum") {
        Some(Chain::Ethereum)
    } else if matches.get_flag("solana") {
        Some(Chain::Solana)
    } else {
        Some(Chain::Bitcoin)
    };

    // 2) Extract vanity mode
    let vanity_mode = if matches.get_flag("regex") {
        Some(VanityMode::Regex)
    } else if matches.get_flag("anywhere") {
        Some(VanityMode::Anywhere)
    } else if matches.get_flag("suffix") {
        Some(VanityMode::Suffix)
    } else {
        Some(VanityMode::Prefix)
    };

    // 3) Threads
    let threads = matches
        .get_one::<String>("threads")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(8);

    // 4) Build CLI-level `VanityFlags`
    let cli_flags = VanityFlags {
        force_flags: matches.get_flag("force-flags"),
        is_case_sensitive: matches.get_flag("case-sensitive"),
        disable_fast_mode: matches.get_flag("disable-fast-mode"),
        output_file_name: matches.get_one::<String>("output-file").cloned(),
        vanity_mode,
        chain,
        threads,
        backend: matches
            .get_one::<String>("backend")
            .and_then(|value| value.parse::<VanityBackend>().ok())
            .or(Some(VanityBackend::Hybrid)),
        gpu_batch_size: matches
            .get_one::<String>("gpu-batch-size")
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0),
    };

    // 5) Figure out if user gave a single pattern or a file
    if let Some(path) = matches.get_one::<String>("input-file") {
        (cli_flags, PatternsSource::InputFile(path.to_string()))
    } else {
        let string = matches.get_one::<String>("string");
        (
            cli_flags,
            PatternsSource::SingleString(string.unwrap_or(&String::new()).to_string()),
        )
    }
}

/// Combines CLI-level flags with file-based flags, giving priority to CLI flags if `force_flags` is set.
///
/// # Arguments
/// - `file_flags`: The `VanityFlags` object derived from the input file.
///
/// # Returns
/// - A unified `VanityFlags` object that combines CLI and file flags.
///
/// # Behavior
/// - If `force_flags` is `true`, the CLI flags override all file-based flags.
/// - Otherwise, the flags are merged, with file-based flags taking precedence where applicable.
///
/// # Example
/// ```rust
/// use btc_vanity::flags::VanityFlags;
///
/// let cli_flags = VanityFlags {
///     threads: 8,
///     force_flags: true,
///     ..Default::default()
/// };
/// let file_flags = VanityFlags {
///     threads: 4,
///     ..Default::default()
/// };
///
/// let unified_flags = cli_flags.unify(&file_flags);
/// assert_eq!(unified_flags.threads, 8); // CLI flags take precedence.
/// ```
impl VanityFlags {
    pub fn unify(&self, file_flags: &VanityFlags) -> VanityFlags {
        if self.force_flags {
            // If CLI has force_flags = true, ignore the file-based flags
            self.clone()
        } else {
            VanityFlags {
                threads: if file_flags.threads > 0 {
                    file_flags.threads
                } else {
                    self.threads
                },
                output_file_name: file_flags
                    .output_file_name
                    .clone()
                    .or_else(|| self.output_file_name.clone()),

                force_flags: self.force_flags,
                is_case_sensitive: file_flags.is_case_sensitive,
                disable_fast_mode: file_flags.disable_fast_mode,

                vanity_mode: file_flags.vanity_mode.or(self.vanity_mode), // Use `file_flags` if Some, otherwise fall back to `self`.

                chain: file_flags.chain.or(self.chain), // Use `file_flags` if Some, otherwise fall back to `self`.
                backend: file_flags.backend.or(self.backend),
                gpu_batch_size: file_flags.gpu_batch_size.or(self.gpu_batch_size),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_cli, PatternsSource, VanityFlags};
    use crate::cli::cli;
    use crate::{VanityBackend, VanityMode};

    #[test]
    fn test_parse_cli_uses_threads_default_from_cli() {
        let matches = cli()
            .try_get_matches_from(["btc-vanity", "abc"])
            .expect("CLI parse should succeed");
        let (flags, source) = parse_cli(matches);

        assert_eq!(flags.threads, 8);
        assert!(matches!(source, PatternsSource::SingleString(_)));
    }

    #[test]
    fn test_unify_uses_file_threads_when_provided() {
        let cli_flags = VanityFlags {
            threads: 8,
            force_flags: false,
            vanity_mode: Some(VanityMode::Prefix),
            backend: Some(VanityBackend::Auto),
            ..VanityFlags::default()
        };
        let file_flags = VanityFlags {
            threads: 3,
            vanity_mode: Some(VanityMode::Anywhere),
            backend: Some(VanityBackend::Gpu),
            ..VanityFlags::default()
        };

        let unified = cli_flags.unify(&file_flags);
        assert_eq!(unified.threads, 3);
        assert_eq!(unified.vanity_mode, Some(VanityMode::Anywhere));
        assert_eq!(unified.backend, Some(VanityBackend::Gpu));
    }

    #[test]
    fn test_unify_keeps_cli_threads_when_file_threads_not_provided() {
        let cli_flags = VanityFlags {
            threads: 8,
            force_flags: false,
            ..VanityFlags::default()
        };
        let file_flags = VanityFlags {
            threads: 0,
            ..VanityFlags::default()
        };

        let unified = cli_flags.unify(&file_flags);
        assert_eq!(unified.threads, 8);
    }

    #[test]
    fn test_parse_cli_accepts_hybrid_backend_alias() {
        let matches = cli()
            .try_get_matches_from(["btc-vanity", "--backend", "both", "abc"])
            .expect("CLI parse should succeed");
        let (flags, source) = parse_cli(matches);

        assert_eq!(flags.backend, Some(VanityBackend::Hybrid));
        assert!(matches!(source, PatternsSource::SingleString(_)));
    }

    #[test]
    fn test_parse_cli_reads_gpu_batch_size_override() {
        let matches = cli()
            .try_get_matches_from(["btc-vanity", "--gpu-batch-size", "262144", "abc"])
            .expect("CLI parse should succeed");
        let (flags, source) = parse_cli(matches);

        assert_eq!(flags.gpu_batch_size, Some(262_144));
        assert!(matches!(source, PatternsSource::SingleString(_)));
    }

    #[test]
    fn test_parse_cli_defaults_backend_to_hybrid() {
        let matches = cli()
            .try_get_matches_from(["btc-vanity", "abc"])
            .expect("CLI parse should succeed");
        let (flags, source) = parse_cli(matches);

        assert_eq!(flags.backend, Some(VanityBackend::Hybrid));
        assert!(matches!(source, PatternsSource::SingleString(_)));
    }
}
