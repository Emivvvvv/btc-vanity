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
//! # Some Usage Examples
//!
//! Find a vanity address with prefix "Emiv" and appends the wallet details to -wallet.txt
//! (if there is no wallet.txt it crates a new one)
//! ```bash
//! $ btc-vanity -o wallet.txt Emiv
//! ```
//!
//! Gets all the inputs and flags (if available) from the inputs.txt text file
//! sets the vanity mode anywhere for the strings which don't have any vanity mode flag
//! and appends all the wallet details to -wallets.txt with using 8 threads
//! (if there is no wallets.txt it crates a new one)
//! ```bash
//! $ btc-vanity -i inputs.txt -o wallets.txt -t 8 -a
//! ```
//!
//! Gets all the inputs and flags (if available) from the inputs.txt text file
//! overrides all flags with the vanity mode to suffix, if a strings has its own
//! -o <text file> flag it ignores it because of -f flag adn prints all the wallet details
//! to stdout.
//! ```bash
//! $ btc-vanity -f -s -i inputs.txt
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
        )
        .arg(
            Arg::new("ethereum")
                .long("eth")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["bitcoin", "solana"])
        )
        .arg(
            Arg::new("solana")
                .long("sol")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["bitcoin", "ethereum"])
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
                .help("Path to a file containing strings to match addresses. \
                      \nImportant: Each string should be written on a separate line."),
        )
        .arg(
            Arg::new("output-file")
                .short('o')
                .long("output-file")
                .value_name("FILE")
                .help("Creates or appends found wallet(s) to the specified file."),
        )
        .arg(
            Arg::new("force-flags")
                .short('f')
                .long("force-flags")
                .action(ArgAction::SetTrue)
                .help("Overrides the flags in the input file. \
                      \nIf set, this will enforce CLI-provided flags. \
                      \nNote: CLI -f is stronger than input-file -f."),
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
                .help("Finds a vanity address with the specified string as a prefix. [default]"),
        )
        .arg(
            Arg::new("suffix")
                .short('s')
                .long("suffix")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "anywhere", "regex"])
                .help("Finds a vanity address with the specified string as a suffix."),
        )
        .arg(
            Arg::new("anywhere")
                .short('a')
                .long("anywhere")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "suffix", "regex"])
                .help("Finds a vanity address containing the specified string anywhere in the address."),
        )
        .arg(
            Arg::new("regex")
                .short('r')
                .long("regex")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["prefix", "suffix", "anywhere"])
                .long_help(
                    "Specifies a regex pattern for your desired vanity address. \
                    \nSupports common regex syntax, such as anchors (^, $), character classes, and wildcards. \
                    \nExample: '^1abc.*xyz$' matches addresses starting with '1abc' and ending with 'xyz'. \
                    \nNote: If your pattern starts with '^' but not '^1', '1' will automatically be prepended \
                    \n(e.g., '^E' becomes '^1E'). \
                    \nOnly Base58 characters (excluding '0', 'I', 'O', 'l') are valid in matches. \
                    \nRegex mode has no length restriction. However, long or restrictive patterns may \
                    \nsignificantly increase search time and could make finding a match impossible.",
                ),
        )
        .arg(
            Arg::new("threads")
                .short('t')
                .long("threads")
                .value_name("N")
                .default_value("8")
                .help("Sets the number of threads to use."),
        )
        .arg(
            Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .action(ArgAction::SetTrue)
                .help("Enables case-sensitive matching for addresses."),
        )
        .arg(
            Arg::new("disable-fast-mode")
                .short('d')
                .long("disable-fast")
                .action(ArgAction::SetTrue)
                .help("Disables fast mode, allowing for prefixes longer than 4 characters."),
        )
}
