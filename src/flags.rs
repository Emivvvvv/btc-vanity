//! # Cli and Input File Flags Module
//!
//! This module is used for getting flags and file names from the cli
//! and change flags for each string iteration if any other flags set in input file.

use crate::file::{get_strings_and_flags_from_file, FileFlags};
use crate::vanity_addr_generator::VanityMode;
use clap::Command;

/// This struct is used to save the cli flags
pub struct CliFlags {
    threads: u64,
    strings: Vec<String>,
    flags: Vec<FileFlags>,
    force_flags: bool,
    is_case_sensitive: bool,
    is_fast_disabled: bool,
    output_file_name: String,
    vanity_mode: VanityMode,
}

impl CliFlags {
    pub fn get_strings(&self) -> &Vec<String> {
        &self.strings
    }

    pub fn get_threads(&self) -> u64 {
        self.threads
    }
}

/// Gets all the set flags, file names from cli and returns them with CliFlags struct
pub fn get_cli_flags(app: Command) -> CliFlags {
    // Gets all the arguments from the cli.
    let matches = app.get_matches();
    let threads = matches
        .get_one::<String>("threads")
        .expect("This was unexpected :(. Something went wrong while getting -t or --threads arg")
        .trim()
        .parse::<u64>()
        .expect("Threads must be a number!");
    let (strings, flags_vec) = match matches.get_one::<String>("string") {
        Some(string) => (vec![string.to_owned()], vec![FileFlags::use_cli_flags()]),
        None => {
            let file_name = matches.get_one::<String>("input-file").unwrap();
            get_strings_and_flags_from_file(file_name).unwrap()
        }
    };

    let cli_force_flags = matches.get_flag("force-flags");
    let cli_is_case_sensitive = matches.get_flag("case-sensitive");
    let cli_is_fast_disabled = matches.get_flag("disable-fast-mode");
    let cli_output_file_name = match matches.get_one::<String>("output-file") {
        Some(output_file_name) => output_file_name.to_string(),
        None => String::from(""),
    };

    // Sets vanity_mode for searching and mode to predefined decoration strings.
    let cli_vanity_mode = if matches.get_flag("anywhere") {
        VanityMode::Anywhere
    } else if matches.get_flag("suffix") {
        VanityMode::Suffix
    } else {
        VanityMode::Prefix
    };

    CliFlags {
        threads,
        strings,
        flags: flags_vec,
        force_flags: cli_force_flags,
        is_case_sensitive: cli_is_case_sensitive,
        is_fast_disabled: cli_is_fast_disabled,
        output_file_name: cli_output_file_name,
        vanity_mode: cli_vanity_mode,
    }
}

/// This struct is used to save the strings flags for each string in the input file.
/// Each iteration means a new StringFlag structure will be created.
pub struct StringsFlags {
    is_case_sensitive: bool,
    is_fast_disabled: bool,
    output_file_name: String,
    vanity_mode: VanityMode,
}

impl StringsFlags {
    /// Creates a new StringFlags
    fn from(
        is_case_sensitive: bool,
        is_fast_disabled: bool,
        output_file_name: String,
        vanity_mode: VanityMode,
    ) -> Self {
        StringsFlags {
            is_case_sensitive,
            is_fast_disabled,
            output_file_name,
            vanity_mode,
        }
    }

    /// If the use used -f or --force flags in cli
    /// String flags will be same as cli flags
    fn use_cli(cli_args: &CliFlags) -> Self {
        StringsFlags {
            is_case_sensitive: cli_args.is_case_sensitive,
            is_fast_disabled: cli_args.is_fast_disabled,
            output_file_name: cli_args.output_file_name.to_string(),
            vanity_mode: cli_args.vanity_mode,
        }
    }

    pub fn get_vanity_mode(&self) -> VanityMode {
        self.vanity_mode
    }

    pub fn get_case_sensitivity(&self) -> bool {
        self.is_case_sensitive
    }

    pub fn get_output_file_name(&self) -> &String {
        &self.output_file_name
    }

    pub fn get_is_fast_mode_disabled(&self) -> bool {
        self.is_fast_disabled
    }
}

/// Returns A StringFlags depending string's flags that we get from the input file.
/// If -f --force is set in cli it just return StringFlag struct that has the same flags
/// with the cli flags
pub fn get_strings_flags(cli_args: &CliFlags, index: usize) -> StringsFlags {
    match cli_args.force_flags {
        true => StringsFlags::use_cli(cli_args), // Use the provided CLI arguments directly
        false => {
            let flags = &cli_args.flags[index]; // Get flags for the specified index
            let force_flags = flags.force_flags; // Check if force flags are set
            let string_vanity_mode = match flags.vanity_mode {
                Some(vanity_mode) => vanity_mode, // Use specified vanity mode if available
                None => cli_args.vanity_mode,     // Otherwise, use CLI argument vanity mode
            };
            let string_output_file_name = match &flags.output_file_name {
                Some(output_file_name) => output_file_name, // Use specified output file name if available
                None => &cli_args.output_file_name, // Otherwise, use CLI argument output file name
            };
            // Determine case sensitivity based on force_flags and CLI arguments
            let string_is_case_sensitive = if force_flags {
                flags.is_case_sensitive
            } else {
                cli_args.is_case_sensitive || flags.is_case_sensitive
            };
            // Determine fast mode disabling based on force_flags and CLI arguments
            let string_is_fast_disabled = if force_flags {
                flags.disable_fast_mode
            } else {
                cli_args.is_fast_disabled || flags.disable_fast_mode
            };

            // Construct and return the StringsArgs struct
            StringsFlags::from(
                string_is_case_sensitive,
                string_is_fast_disabled,
                string_output_file_name.to_string(),
                string_vanity_mode,
            )
        }
    }
}
