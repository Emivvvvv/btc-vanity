//! # File Reading and Writing Module
//!
//! This module is used for reading multiple strings and flags from files and writing found vanity wallets to desired destination.

use crate::error::FileError;
use crate::vanity_addr_generator::VanityMode;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;

/// This struct is used to get set flags for each string input
/// from the file.
pub struct FileFlags {
    pub force_flags: bool,
    pub is_case_sensitive: bool,
    pub disable_fast_mode: bool,
    pub output_file_name: Option<String>,
    pub vanity_mode: Option<VanityMode>,
}

impl FileFlags {
    /// If there is no flags set for a string in input-file or use just
    /// gave the string from cli this sets the string's flag settings in order
    /// to use cli set flags.
    pub fn use_cli_flags() -> Self {
        FileFlags {
            force_flags: false,
            is_case_sensitive: false,
            disable_fast_mode: false,
            output_file_name: None,
            vanity_mode: None,
        }
    }
}

/// Gets all the flags in the line and returns a FileFlags struct.
/// Each flag must be seperated by space.
pub fn get_flags(line: &str) -> FileFlags {
    let args = line.split(' ').collect::<Vec<_>>();

    if args.len() == 1 {
        return FileFlags::use_cli_flags();
    }

    let force_flags = args.contains(&"-f") || args.contains(&"--force-flags");
    let is_case_sensitive = args.contains(&"-c") || args.contains(&"--case-sensitive");
    let disable_fast_mode = args.contains(&"-d") || args.contains(&"--disable-fast");
    let vanity_option = args.iter().find(|&&arg| {
        arg == "-p"
            || arg == "-s"
            || arg == "-a"
            || arg == "--prefix"
            || arg == "--suffix"
            || arg == "--anywhere"
    });
    let vanity_mode = match vanity_option {
        Some(&vanity) => match vanity {
            "-p" | "--prefix" => Some(VanityMode::Prefix),
            "-s" | "--suffix" => Some(VanityMode::Suffix),
            _ => Some(VanityMode::Anywhere),
        },
        None => None,
    };
    let ofn_index = args
        .iter()
        .position(|&arg| arg == "-o" || arg == "--output-file");
    let output_file_name = ofn_index
        .and_then(|i| args.get(i + 1))
        .map(ToString::to_string);

    FileFlags {
        force_flags,
        is_case_sensitive,
        disable_fast_mode,
        output_file_name,
        vanity_mode,
    }
}

/// Gets all strings and the flags from the input file. Strings and their flags must be in a different lines.
///
/// Example inputs.txt
/// ```txt
/// Emiv -p -c
/// TALA -a
/// 3169
/// test -o test-output.txt
/// ```
pub fn get_strings_and_flags_from_file(
    file_name: &String,
) -> Result<(Vec<String>, Vec<FileFlags>), FileError> {
    let data = fs::read_to_string(file_name)?;
    let lines: Vec<&str> = data.lines().collect::<Vec<_>>();
    let strings: Vec<_> = lines
        .iter()
        .map(|line| {
            let line_split = line.split(' ').collect::<Vec<_>>();
            line_split[0].to_string()
        })
        .collect();
    let flags: Vec<FileFlags> = lines.iter().map(|&string| get_flags(string)).collect();

    Ok((strings, flags))
}

/// If file already exists appends else creates an output text file and writes all the found wallet details.
///
/// Example output.txt
/// ```txt
/// Key pair which their address has the prefix: 'Emiv' (case sensitive)
///
/// private_key (wif): L4RMjXo3AWzBuJTv98ZPoLtPtPP71aLwG7xV5pXodxGzWNZmK6Db
/// public_key (compressed): 03a80e3296e19ffd656210aafe1bc2acb5a41c6a9b9361631c68fb7c9dbd416563
/// address (compressed): 1Emiv6UxeRbAchqLvLEyVXDeL8UkrEUpzd
///
/// Key pair which their address has the string: 'TALA' (case sensitivity disabled)
///
/// private_key (wif): KzMLndRF3EjgLPnuYsQC31sgcvJKkqX2XoebovRPjdYAp5rYPhHm
/// public_key (compressed): 02dc0eaebe451bc868ac0a7806f1ccde356c9a0e296217b684ba9095d9a41cb36a
/// address (compressed): 1G9rGeY13XZoa8CjK3BhaEtqotaLasmbB7
///
/// Key pair which their address has the string: '3169' (case sensitivity disabled)
///
/// private_key (wif): L44VHVPLhD19TDpVzY3oJU5TXANVNpjVzYSB89giP1mUdPy4WexG
/// public_key (compressed): 0349604b1459052a9f0bd993c4b03af4faa8e8e08c733dac129c6bcc1cdbae6057
/// address (compressed): 1PKFWNcazkSKymp3169CSGBsN8gbTGeJDm
///
/// Key pair which their address has the string: 'tala' (case sensitivity disabled)
///
/// Skipping because of error: Custom Error: Your input is not in base58. Don't include zero: '0', uppercase i: 'I', uppercase o: 'O', lowercase L: 'l', in your input!
/// ```
pub fn write_output_file(output_file_name: &String, buffer: &String) -> Result<(), FileError> {
    let ofn_len = output_file_name.len();
    if &output_file_name[ofn_len - 4..ofn_len] != ".txt" {
        return Err(FileError(
            "file must be a text file. ex: output.txt".to_string(),
        ));
    }
    let file_result = OpenOptions::new().append(true).open(output_file_name);
    let mut file = match file_result {
        Ok(file) => file,
        Err(_) => fs::File::create(output_file_name)?,
    };

    file.write_all(buffer.as_bytes())?;
    Ok(())
}
