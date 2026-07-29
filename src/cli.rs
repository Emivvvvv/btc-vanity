//! # Cli With Using Clap Crate
//!
//! This module is used for creating a cli app for btc-vanity with using clap crate
//!
//! # Usage
//!
//! ```bash
//! $ btc-vanity --help
//! ```
//!
//! The CLI tool provides several options to customize your address generation:
//!
//! ```shell
//! $ btc-vanity [OPTIONS] <PATTERN>
//! ```
//!
//! #### Blockchain Selection
//! `--btc`: Generates Bitcoin keypairs and addresses. [default] <br>
//! `--eth`: Generates Ethereum keypairs and addresses. <br>
//! `--sol`: Generates Solana keypairs and addresses. <br>
//!
//! #### General Options
//! `-i, --input-file <FILE>`: Reads patterns and it's flags from the specified file for vanity address generation, with one pattern per line. <br>
//! `-o, --output-file <FILE>`: Saves generated wallet details to the specified file, creating it if it doesn’t exist or appending if it does. <br>
//! `-t, --threads <N>`: Sets the number of threads for address generation. <br>
//! `-b, --backend <BACKEND>`: Selects the execution backend (`auto`, `cpu`, `gpu`, `hybrid`). `both` is accepted as an alias for `hybrid`. <br>
//! `--gpu-batch-size <N>`: Overrides GPU dynamic tuning batch size (only used by GPU/hybrid paths). <br>
//! `--gpu-usage-limit <PERCENT>`: Limits GPU dispatch duty cycle to 1–100%. Defaults to 70% in Hybrid and 100% in GPU mode. <br>
//! `-f, --force-flags`: Forces CLI flags to override any flags specified in the input file, ensuring consistent behavior across all patterns. <br>
//! `-d, --disable-fast`: Disables fast mode to allow longer patterns (5 for BTC and SOL, 16 for ETH), though it may increase search time. <br>
//!
//! #### Matching Options
//! `-p, --prefix`: Matches the pattern as a prefix of the address. [default] <br>
//! `-s, --suffix`: Matches the pattern as a suffix of the address. <br>
//! `-a, --anywhere`: Matches the pattern anywhere in the address. <br>
//! `-r, --regex <REGEX>`: Matches addresses using a regex pattern, supporting advanced customization like anchors and wildcards. <br>
//! `-c, --case-sensitive`: Enables case-sensitive matching, making patterns distinguish between uppercase and lowercase characters. <br>
//!
//! ### Bitcoin CLI Examples
//!
//! Generate a Bitcoin address with prefix `1Emiv` (case-insensitive):
//!
//! ```shell
//! $ btc-vanity Emiv
//! ```
//!
//! Generate a Bitcoin address containing the substring `test` (case-sensitive):
//!
//! ```shell
//! $ btc-vanity -a -c test
//! ```
//!
//! Generate a Bitcoin address using a regex pattern `^1E.*T$`:
//!
//! ```shell
//! $ btc-vanity -r "^1E.*T$"
//! ```
//!
//! Generate multiple Bitcoin addresses and save to wallets.txt:
//!
//! > [!NOTE]
//! > -f flag will override any pattern flags inside the `input-file.txt`.
//! > For example if there line `emiv -s --eth` will become `emiv -p --btc -c`.
//! > The resulting wallet will be printed in `wallets.txt`.
//!
//! ```shell
//! $ btc-vanity -f --btc -p -c -i input-file.txt -o wallets.txt
//! ```
//!
//! Generate an Ethereum address starting with 0xdead with 8 threads:
//!
//! ```shell
//! $ btc-vanity --eth -t 8 dead
//! ```
//!
//! Generate a Solana address ending with 123:
//!
//! ```shell
//! $ btc-vanity --sol -s 123
//! ```

use clap::{Arg, ArgAction, ArgGroup, Command};

/// Runs the clap app to create the CLI
pub fn cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .next_line_help(true)
        .arg(
            Arg::new("bitcoin")
                .long("btc")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["ethereum", "solana"])
                .help_heading("Chain Selection")
                .help("Generates Bitcoin keypairs and addresses. [default]")
        )
        .arg(
            Arg::new("ethereum")
                .long("eth")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["bitcoin", "solana", "case-sensitive"])
                .help_heading("Chain Selection")
                .help("Generates Ethereum keypairs and addresses.")
        )
        .arg(
            Arg::new("solana")
                .long("sol")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["bitcoin", "ethereum"])
                .help_heading("Chain Selection")
                .help("Generates Solana keypairs and addresses.")
        )
        .arg(
            Arg::new("string")
                .index(1)
                .required_unless_present_any(["input-file"])
                .help_heading("Input")
                .help("The pattern or regular expression to match against addresses."),
        )
        .arg(
            Arg::new("input-file")
                .short('i')
                .long("input-file")
                .required_unless_present_any(["string"])
                .value_name("FILE")
                .help_heading("Input")
                .help("Reads one pattern and its optional flags from each line of a file."),
        )
        .arg(
            Arg::new("output-file")
                .short('o')
                .long("output-file")
                .value_name("FILE")
                .help_heading("Input/Output")
                .help("Saves generated wallet details to the specified file, creating it if it doesn’t exist or appending if it does."),
        )
        .arg(
            Arg::new("force-flags")
                .short('f')
                .long("force-flags")
                .action(ArgAction::SetTrue)
                .help_heading("Input/Output")
                .help("Forces CLI flags to override any flags specified in the input file, ensuring consistent behavior across all patterns."),
        )
        .group(
            ArgGroup::new("pattern")
                .args(["prefix", "suffix", "anywhere", "regex"])
                .multiple(false) // Only one pattern type can be used
                .required(false), // Not required globally
        )
        .arg(
            Arg::new("prefix")
                .short('p')
                .long("prefix")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["suffix", "anywhere", "regex"])
                .help_heading("Match Mode")
                .help("Matches the pattern as a prefix of the address. [default]"),
        )
        .arg(
            Arg::new("suffix")
                .short('s')
                .long("suffix")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "anywhere", "regex"])
                .help_heading("Match Mode")
                .help("Matches the pattern as a suffix of the address."),
        )
        .arg(
            Arg::new("anywhere")
                .short('a')
                .long("anywhere")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "suffix", "regex"])
                .help_heading("Match Mode")
                .help("Matches the pattern anywhere in the address."),
        )
        .arg(
            Arg::new("regex")
                .short('r')
                .long("regex")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "suffix", "anywhere"])
                .help_heading("Match Mode")
                .help("Matches addresses using a regex pattern, supporting advanced customization like anchors and wildcards."),
        )
        .arg(
            Arg::new("threads")
                .short('t')
                .long("threads")
                .value_name("N")
                .value_parser(|value: &str| match value.parse::<usize>() {
                    Ok(0) => Err("thread count must be at least 1".to_owned()),
                    Ok(threads) => Ok(threads),
                    Err(_) => Err("thread count must be a positive integer".to_owned()),
                })
                .help_heading("Performance")
                .help("Sets the CPU worker count. Defaults to available CPU parallelism."),
        )
        .arg(
            Arg::new("backend")
                .short('b')
                .long("backend")
                .value_name("BACKEND")
                .value_parser(["auto", "cpu", "gpu", "hybrid", "both"])
                .help_heading("Performance")
                .help("Selects the execution backend: auto, cpu, gpu, or hybrid (both). [default: hybrid]"),
        )
        .arg(
            Arg::new("gpu-batch-size")
                .long("gpu-batch-size")
                .value_name("N")
                .help_heading("Performance")
                .help("Overrides GPU batch size for GPU and hybrid backends. Larger values may improve throughput but reduce responsiveness."),
        )
        .arg(
            Arg::new("gpu-usage-limit")
                .long("gpu-usage-limit")
                .value_name("PERCENT")
                .value_parser(|value: &str| match value.parse::<u8>() {
                    Ok(limit @ 1..=100) => Ok(limit),
                    _ => Err("GPU usage limit must be between 1 and 100".to_owned()),
                })
                .help_heading("Performance")
                .help("Limits the GPU dispatch duty cycle to 1-100%. Defaults to 70% in hybrid mode and 100% in GPU mode."),
        )
        .arg(
            Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .action(ArgAction::SetTrue)
                .help_heading("Matching Behavior")
                .help("Enables case-sensitive matching, making patterns distinguish between uppercase and lowercase characters."),
        )
        .arg(
            Arg::new("disable-fast-mode")
                .short('d')
                .long("disable-fast")
                .action(ArgAction::SetTrue)
                .help_heading("Matching Behavior")
                .help("Removes the short-pattern guard (5 characters for BTC/SOL and 16 for ETH); absolute limits remain."),
        )
}
