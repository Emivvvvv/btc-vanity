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
//!
//! #### General Options
//! `-i, --input-file <FILE>`: Reads patterns and it's flags from the specified file for vanity address generation, with one pattern per line. <br>
//! `-o, --output-file <FILE>`: Saves generated wallet details to the specified file, creating it if it doesn’t exist or appending if it does. <br>
//! `-t, --threads <N>`: Sets the number of threads for address generation. <br>
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
                .conflicts_with_all(["ethereum"])
                .help("Generates Bitcoin keypairs and addresses. [default]")
        )
        .arg(
            Arg::new("ethereum")
                .long("eth")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["bitcoin", "case-sensitive"])
                .help("Generates Ethereum keypairs and addresses.")
        )
        .arg(
            Arg::new("string")
                .index(1)
                .required_unless_present_any(["input-file"])
                .help("The string (or regex) used to match Bitcoin vanity addresses."),
        )
        .arg(
            Arg::new("input-file")
                .short('i')
                .long("input-file")
                .required_unless_present_any(["string"])
                .value_name("FILE")
                .help("Reads patterns and it's flags from the specified file for vanity address generation, with one pattern and it's flags per line."),
        )
        .arg(
            Arg::new("output-file")
                .short('o')
                .long("output-file")
                .value_name("FILE")
                .help("Saves generated wallet details to the specified file, creating it if it doesn’t exist or appending if it does."),
        )
        .arg(
            Arg::new("force-flags")
                .short('f')
                .long("force-flags")
                .action(ArgAction::SetTrue)
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
                .help("Matches the pattern as a prefix of the address. [default]"),
        )
        .arg(
            Arg::new("suffix")
                .short('s')
                .long("suffix")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "anywhere", "regex"])
                .help("Matches the pattern as a suffix of the address."),
        )
        .arg(
            Arg::new("anywhere")
                .short('a')
                .long("anywhere")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "suffix", "regex"])
                .help("Matches the pattern anywhere in the address."),
        )
        .arg(
            Arg::new("regex")
                .short('r')
                .long("regex")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "suffix", "anywhere"])
                .help("Matches addresses using a regex pattern, supporting advanced customization like anchors and wildcards."),
        )
        .arg(
            Arg::new("threads")
                .short('t')
                .long("threads")
                .value_name("N")
                .default_value("8")
                .help("Sets the number of threads for address generation."),
        )
        .arg(
            Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .action(ArgAction::SetTrue)
                .help("Enables case-sensitive matching, making patterns distinguish between uppercase and lowercase characters."),
        )
        .arg(
            Arg::new("disable-fast-mode")
                .short('d')
                .long("disable-fast")
                .action(ArgAction::SetTrue)
                .help("Disables fast mode to allow longer patterns (5 for BTC and SOL, 16 for ETH), though it may increase search time."),
        )
}
