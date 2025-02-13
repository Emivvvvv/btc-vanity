//! # File Reading and Writing Module
//!
//! This module provides functionality for:
//! - Parsing input files containing vanity patterns and flags.
//! - Writing generated vanity wallet details to output files.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::error::VanityError;
use crate::flags::VanityFlags;
use crate::vanity_addr_generator::chain::Chain;
use crate::VanityMode;

/// Represents a single line item from an input file,
/// containing a vanity pattern and associated flags.
#[derive(Debug, Clone)]
pub struct FileLineItem {
    /// The vanity pattern to match (e.g., "emiv").
    pub pattern: String,
    /// The associated `VanityFlags` configuration.
    pub flags: VanityFlags,
}

/// Parses a single line from an input file into a [FileLineItem].
///
/// # Arguments
/// - `line`: A string slice representing a line from the input file.
///
/// # Returns
/// - `Some(FileLineItem)` if the line is valid and parsable.
/// - `None` if the line is empty or starts with a comment (`#`).
fn parse_line(line: &str) -> Option<FileLineItem> {
    if line.starts_with('#') {
        return None;
    }

    let mut tokens = line.split_whitespace();

    // The first token is the pattern
    let pattern = tokens.next()?.to_string();
    // The rest are "flags"
    let flags_vec: Vec<&str> = tokens.collect();

    // If no flags, then everything is default
    if flags_vec.is_empty() {
        return Some(FileLineItem {
            pattern,
            flags: VanityFlags {
                force_flags: false,
                is_case_sensitive: false,
                disable_fast_mode: false,
                output_file_name: None,
                vanity_mode: None,
                chain: None,
                threads: 16,
            },
        });
    }

    let is_case_sensitive = flags_vec.contains(&"-c") || flags_vec.contains(&"--case-sensitive");
    let disable_fast_mode = flags_vec.contains(&"-d") || flags_vec.contains(&"--disable-fast");

    // chain
    let chain = if flags_vec.contains(&"--eth") {
        Some(Chain::Ethereum)
    } else if flags_vec.contains(&"--sol") {
        Some(Chain::Solana)
    } else if flags_vec.contains(&"--btc") {
        Some(Chain::Bitcoin)
    } else {
        None
    };

    // vanity mode
    let vanity_mode = if flags_vec.contains(&"-r") || flags_vec.contains(&"--regex") {
        Some(VanityMode::Regex)
    } else if flags_vec.contains(&"-a") || flags_vec.contains(&"--anywhere") {
        Some(VanityMode::Anywhere)
    } else if flags_vec.contains(&"-s") || flags_vec.contains(&"--suffix") {
        Some(VanityMode::Suffix)
    } else if flags_vec.contains(&"-p") || flags_vec.contains(&"--prefix") {
        Some(VanityMode::Prefix)
    } else {
        None
    };

    // output file name: look for `-o` or `--output-file` plus the next token
    let mut output_file_name: Option<String> = None;
    for (i, &flag) in flags_vec.iter().enumerate() {
        if flag == "-o" || flag == "--output-file" {
            if let Some(next_flag) = flags_vec.get(i + 1) {
                output_file_name = Some(next_flag.to_string());
            }
        }
    }

    Some(FileLineItem {
        pattern,
        flags: VanityFlags {
            force_flags: false,
            is_case_sensitive,
            disable_fast_mode,
            output_file_name,
            vanity_mode,
            chain,
            threads: 0,
        },
    })
}

/// Reads and parses an input file, converting each line into a [FileLineItem].
///
/// # Arguments
/// - `path`: A string slice representing the file path.
///
/// # Returns
/// - `Ok(Vec<FileLineItem>)`: A vector of parsed [FileLineItem] objects.
/// - `Err(VanityError)`: An error if the file cannot be read or parsed.
///
/// # Errors
/// - Returns `VanityError::FileError` if the file cannot be read.
pub fn parse_input_file(path: &str) -> Result<Vec<FileLineItem>, VanityError> {
    let contents = fs::read_to_string(path)?;
    let mut items = Vec::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(item) = parse_line(line) {
            items.push(item);
        }
    }
    Ok(items)
}

/// Writes a string buffer to an output file. If the file does not exist, it will be created.
///
/// # Arguments
/// - `output_path`: The path to the output file.
/// - `buffer`: The content to write to the file.
///
/// # Returns
/// - `Ok(())` on successful write.
/// - `Err(VanityError)` if the operation fails.
///
/// # Errors
/// - Returns `VanityError::FileError` if the operation fails,
///   such as due to invalid input or a write failure.
pub fn write_output_file(output_path: &Path, buffer: &str) -> Result<(), VanityError> {
    // Attempt to open the file in append mode
    let file_result = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output_path);
    let mut file = match file_result {
        Ok(file) => file,
        Err(e) => {
            return Err(VanityError::FileError(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open or create file: {}", e),
            )))
        }
    };

    // Write the buffer to the file
    if let Err(e) = file.write_all(buffer.as_bytes()) {
        return Err(VanityError::FileError(io::Error::new(
            io::ErrorKind::WriteZero,
            format!("Failed to write to file: {}", e),
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_input_file, parse_line};
    use crate::VanityMode;

    #[test]
    fn test_parse_line_with_valid_flags() {
        let line = "test -p -c";
        let item = parse_line(line).expect("Failed to parse valid line");

        assert_eq!(item.pattern, "test");
        assert_eq!(item.flags.vanity_mode, Some(VanityMode::Prefix));
        assert!(item.flags.is_case_sensitive);
    }

    #[test]
    fn test_parse_line_with_invalid_flags() {
        let line = "test -z";
        let item = parse_line(line).expect("Failed to parse line with invalid flags");

        assert_eq!(item.pattern, "test");
        assert!(item.flags.vanity_mode.is_none());
    }

    #[test]
    fn test_parse_line_with_no_flags() {
        let line = "test";
        let item = parse_line(line).expect("Failed to parse line without flags");

        assert_eq!(item.pattern, "test");
        assert!(item.flags.vanity_mode.is_none());
        assert!(!item.flags.is_case_sensitive);
        assert!(!item.flags.disable_fast_mode);
    }

    #[test]
    fn test_parse_line_with_output_file_flag() {
        let line = "test -p -o output.txt";
        let item = parse_line(line).expect("Failed to parse line with output file flag");

        assert_eq!(item.pattern, "test");
        assert_eq!(item.flags.vanity_mode, Some(VanityMode::Prefix));
        assert_eq!(item.flags.output_file_name, Some("output.txt".to_string()));
    }

    #[test]
    fn test_parse_empty_line() {
        let line = "";
        let item = parse_line(line);

        assert!(item.is_none(), "Empty line should not be parsed");
    }

    #[test]
    fn test_parse_comment_line() {
        let line = "# This is a comment";
        let item = parse_line(line);

        assert!(item.is_none(), "Comment line should not be parsed");
    }

    #[test]
    fn test_parse_input_file_with_valid_lines() {
        // Mock file content
        let file_content = "test -p\nexample -s\nanywhere -a";
        let file_path = "test_valid_input.txt";
        std::fs::write(file_path, file_content).expect("Failed to create mock input file");

        // Parse the file
        let result = parse_input_file(file_path);
        assert!(result.is_ok(), "Failed to parse valid input file");

        let items = result.unwrap();
        assert_eq!(items.len(), 3);

        assert_eq!(items[0].pattern, "test");
        assert_eq!(items[0].flags.vanity_mode, Some(VanityMode::Prefix));

        assert_eq!(items[1].pattern, "example");
        assert_eq!(items[1].flags.vanity_mode, Some(VanityMode::Suffix));

        assert_eq!(items[2].pattern, "anywhere");
        assert_eq!(items[2].flags.vanity_mode, Some(VanityMode::Anywhere));

        // Clean up
        std::fs::remove_file(file_path).expect("Failed to delete mock input file");
    }

    #[test]
    fn test_parse_input_file_with_invalid_lines() {
        // Mock file content
        let file_content = "test -z\nexample --invalid";
        let file_path = "test_invalid_input.txt";
        std::fs::write(file_path, file_content).expect("Failed to create mock input file");

        // Parse the file
        let result = parse_input_file(file_path);
        assert!(result.is_ok(), "Failed to parse file with invalid lines");

        let items = result.unwrap();
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].pattern, "test");
        assert!(items[0].flags.vanity_mode.is_none());

        assert_eq!(items[1].pattern, "example");
        assert!(items[1].flags.vanity_mode.is_none());

        // Clean up
        std::fs::remove_file(file_path).expect("Failed to delete mock input file");
    }

    #[test]
    fn test_parse_input_file_with_empty_lines() {
        // Mock file content
        let file_content = "\n\n";
        let file_path = "test_empty_lines.txt";
        std::fs::write(file_path, file_content).expect("Failed to create mock input file");

        // Parse the file
        let result = parse_input_file(file_path);
        assert!(result.is_ok(), "Failed to parse file with empty lines");

        let items = result.unwrap();
        assert!(
            items.is_empty(),
            "Parsed items should be empty for a file with only empty lines"
        );

        // Clean up
        std::fs::remove_file(file_path).expect("Failed to delete mock input file");
    }

    #[test]
    fn test_parse_input_file_with_invalid_path() {
        // Non-existent file path
        let file_path = "non_existent_file.txt";

        // Parse the file
        let result = parse_input_file(file_path);
        assert!(
            result.is_err(),
            "Parsing a non-existent file should return an error"
        );

        if let Err(err) = result {
            assert!(
                err.to_string().contains("No such file"),
                "Unexpected error message: {}",
                err
            );
        }
    }

    #[test]
    fn test_parse_input_file_with_missing_flags() {
        // Mock file content
        let file_content = "test\nexample\nmissing_flags";
        let file_path = "test_missing_flags.txt";
        std::fs::write(file_path, file_content).expect("Failed to create mock input file");

        // Parse the file
        let result = parse_input_file(file_path);
        assert!(result.is_ok(), "Failed to parse file with missing flags");

        let items = result.unwrap();
        assert_eq!(items.len(), 3);

        assert_eq!(items[0].pattern, "test");
        assert!(items[0].flags.vanity_mode.is_none());

        assert_eq!(items[1].pattern, "example");
        assert!(items[1].flags.vanity_mode.is_none());

        assert_eq!(items[2].pattern, "missing_flags");
        assert!(items[2].flags.vanity_mode.is_none());

        // Clean up
        std::fs::remove_file(file_path).expect("Failed to delete mock input file");
    }
}
