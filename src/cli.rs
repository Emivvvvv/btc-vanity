//! # Cli With Using Clap Crate
//!
//! This module is used for creating a cli app for btc-vanity with using clap crate
//!
//! # Usage
//!
//! ```bash
//! $ btc-vanity --help
//! A bitcoin vanity address generator written with the Rust programming language.
//!
//! Usage: btc-vanity [OPTIONS] [string]
//!
//! Arguments:
//! [string]  String used to match addresses.
//!
//! Options:
//! -i, --input-file <input-file>    File with strings to match addresses with.
//! Important: Write every string in a separate line.
//! -f, --force-flags                Use this flag to override the flags in the input file
//! or use in file to override cli flags for only that string.
//! Note: Cli -f is stronger than input-file -f.
//! -o, --output-file <output-file>  Crates a file that contains found wallet/s.
//! -p, --prefix                     Finds a vanity address which has 'string' prefix. [default]
//! -s, --suffix                     Finds a vanity address which has 'string' suffix.
//! -a, --anywhere                   Finds a vanity address which includes 'string' at any part of the address.
//! -t, --threads <threads>          Number of threads to be used. [default: 16]
//! -c, --case-sensitive             Use case sensitive comparison to match addresses.
//! -d, --disable-fast               Disables fast mode to find a prefix more than 4 characters.
//! -h, --help                       Print help
//! -V, --version                    Print version
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
//! overrides all flags with the vanity mode to suffix, if a strings has it's own
//! -o <text' file> flag it ignores it because of -f flag adn prints all the wallet details
//! to stdout.
//! ```bash
//! $ btc-vanity -f -s -i inputs.txt
//! ```

use clap;

/// Runs the clap app in order to use cli
pub fn cli() -> clap::Command {
    clap::Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::new("string")
                .index(1)
                .required_unless_present_any(["input-file"])
                .help("String used to match addresses."),
        )
        .arg(
            clap::Arg::new("input-file")
                .short('i')
                .long("input-file")
                .required_unless_present_any(["string"])
                .help("File with strings to match addresses with.\nImportant: Write every string in a separate line.")
        )
        .arg(
            clap::Arg::new("force-flags")
                .short('f')
                .long("force-flags")
                .action(clap::ArgAction::SetTrue)
                .help("Use this flag to override the flags in the input file\nor use in file to override cli flags for only that string.\nNote: Cli -f is stronger than input-file -f.")
        )
        .arg(
            clap::Arg::new("output-file")
                .short('o')
                .long("output-file")
                .help("Crates a file that contains found wallet/s."),
        )
        .arg(
            clap::Arg::new("prefix")
                .conflicts_with("suffix")
                .conflicts_with("anywhere")
                .short('p')
                .long("prefix")
                .action(clap::ArgAction::SetTrue)
                .help("Finds a vanity address which has 'string' prefix. [default]")
        )
        .arg(
            clap::Arg::new("suffix")
                .conflicts_with("prefix")
                .conflicts_with("anywhere")
                .short('s')
                .long("suffix")
                .action(clap::ArgAction::SetTrue)
                .help("Finds a vanity address which has 'string' suffix.")
        )
        .arg(
            clap::Arg::new("anywhere")
                .short('a')
                .long("anywhere")
                .action(clap::ArgAction::SetTrue)
                .help("Finds a vanity address which includes 'string' at any part of the address.")
        )
        .arg(
            clap::Arg::new("threads")
                .short('t')
                .long("threads")
                .default_value("16")
                .help("Number of threads to be used."),
        )
        .arg(
            clap::Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .action(clap::ArgAction::SetTrue)
                .help("Use case sensitive comparison to match addresses."),
        )
        .arg(
            clap::Arg::new("disable-fast-mode")
                .short('d')
                .long("disable-fast")
                .action(clap::ArgAction::SetTrue)
                .help("Disables fast mode to find a prefix more than 4 characters."),
        )
        // There is no test features in v1.0.0
        // .arg(
        //     clap::Arg::new("test-features")
        //         .short('x')
        //         .long("test-features")
        //         .action(clap::ArgAction::SetTrue)
        //         .help("Runs the program with test features. (Use in order to use new engine)"),
        // )
}