use std::process::Command;
use std::fs;
use std::io::Write;

use tempfile::NamedTempFile;

#[test]
fn test_cli_with_prefix() {
    // Run the CLI with a prefix
    let result = Command::new("./target/debug/btc-vanity")
        .args(["-p", "tst"]) // Prefix flag and prefix
        .output()
        .expect("Failed to execute CLI command");

    // Check that the CLI exits successfully
    assert!(
        result.status.success(),
        "CLI failed with error: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Verify that the output contains the expected prefix
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.to_ascii_lowercase().contains("address (compressed): 1tst"),
        "CLI output does not contain expected prefix: {}",
        stdout
    );
}

#[test]
fn test_cli_with_regex() {
    // Run the CLI with a regex pattern
    let result = Command::new("./target/debug/btc-vanity")
        .args(["-r", "^1E.*T$"]) // Regex flag and pattern
        .output()
        .expect("Failed to execute CLI command");

    // Check that the CLI exits successfully
    assert!(
        result.status.success(),
        "CLI failed with error: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Verify that the output matches the regex
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("address (compressed): 1E") && stdout.contains("regex: '^1E.*T$'"),
    );
}

#[test]
fn test_cli_with_output_file() {
    let output_file = "test_output.txt";

    // Run the CLI with an output file flag
    let result = Command::new("./target/debug/btc-vanity")
        .args(["-o", output_file, "tst"]) // Output file flag and pattern
        .output()
        .expect("Failed to execute CLI command");

    // Check that the CLI exits successfully
    assert!(
        result.status.success(),
        "CLI failed with error: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Check that the output file exists
    let output = fs::read_to_string(output_file).expect("Failed to read output file");
    assert!(
        output.to_ascii_lowercase().contains("address (compressed): 1tst"),
        "Output file does not contain expected data: {}",
        output
    );

    // Clean up the temporary file
    fs::remove_file(output_file).expect("Failed to delete temporary output file");
}

#[test]
fn test_cli_with_case_sensitivity() {
    // Run the CLI with case-sensitive flag
    let result = Command::new("./target/debug/btc-vanity")
        .args(["-c", "-a", "TST"]) // Case-sensitive flag and pattern
        .output()
        .expect("Failed to execute CLI command");

    // Check that the CLI exits successfully
    assert!(
        result.status.success(),
        "CLI failed with error: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Verify that the output matches the case-sensitive pattern
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("TST"),
        "CLI output does not match expected case-sensitive pattern: {}",
        stdout
    );
}

#[test]
fn test_cli_missing_required_arguments() {
    // Run the CLI without any arguments
    let result = Command::new("./target/debug/btc-vanity")
        .output()
        .expect("Failed to execute CLI command");

    // Check that the CLI exits with an error
    assert!(
        !result.status.success(),
        "CLI succeeded unexpectedly"
    );

    // Verify that the error message is printed
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("the following required arguments were not provided"),
        "CLI error message is incorrect: {}",
        stderr
    );
}

#[test]
fn test_cli_with_input_file_tempfile() {
    // Create a temporary input file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
    writeln!(temp_file, "t1 -p\nT2 -c -p").expect("Failed to write to temporary file");

    // Get the file path
    let input_file_path = temp_file.path().to_str().expect("Failed to get file path");

    // Run the CLI with the input file
    let result = Command::new("./target/debug/btc-vanity")
        .args(["-i", input_file_path]) // Input file flag
        .output()
        .expect("Failed to execute CLI command");

    // Check that the CLI exits successfully
    assert!(
        result.status.success(),
        "CLI failed with error: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Verify that the output contains the expected results
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.to_lowercase().contains("address (compressed): 1t1") && stdout.contains("address (compressed): 1T2"),
        "CLI output does not contain expected results: {}",
        stdout
    );
}

