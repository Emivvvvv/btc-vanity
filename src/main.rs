use btc_vanity::cli::cli;
use btc_vanity::decoration::get_decoration_strings;
use btc_vanity::file::write_output_file;
use btc_vanity::flags::{get_cli_flags, get_strings_flags, Chain, CliFlags};
use btc_vanity::keys_and_address::BitcoinKeyPair;
use btc_vanity::vanity_addr_generator::vanity_addr::{VanityAddr, VanityMode};

use clap::error::ErrorKind;
use std::fmt::Write;
use std::time::Instant;

/// Validates and parses CLI arguments
fn parse_cli() -> CliFlags {
    let app = cli();
    match app.try_get_matches() {
        Ok(matches) => get_cli_flags(matches),
        Err(err) => {
            if err.kind() == ErrorKind::MissingRequiredArgument {
                eprintln!(
                    "error: the following required arguments were not provided:\n  --input-file <input-file> OR <string>\n"
                );
                eprintln!("Usage: btc-vanity [OPTIONS] <string>");
            } else {
                eprintln!("{}", err);
            }
            std::process::exit(1);
        }
    }
}

/// Prints the initial search message
fn print_initial_message(
    chain: Chain,
    threads: u64,
    vanity_mode_str: &str,
    string: &str,
    case_sensitive_str: &str,
) {
    println!(
        "Searching key pair for {} chain where the address {}: '{}' {} with {} threads.\n",
        chain, vanity_mode_str, string, case_sensitive_str, threads
    );
}

/// Handles the vanity address generation and returns the result as a formatted string
fn generate_vanity_address(
    string: &str,
    threads: u64,
    string_flags: &CliFlags,
) -> Result<String, String> {
    let start = Instant::now();
    let result = match string_flags.vanity_mode {
        VanityMode::Regex => VanityAddr::generate_regex::<BitcoinKeyPair>(string, threads),
        _ => VanityAddr::generate::<BitcoinKeyPair>(
            string,
            threads,
            string_flags.is_case_sensitive,
            !string_flags.is_fast_disabled,
            string_flags.vanity_mode,
        ),
    };
    let seconds = start.elapsed().as_secs_f64();

    match result {
        Ok(res) => {
            println!("FOUND IN {:.4} SECONDS!\n", seconds);

            let formatted_private_key_hex =
                res.get_private_key()
                    .to_bytes()
                    .iter()
                    .fold(String::new(), |mut acc, byte| {
                        write!(&mut acc, "{:02X}", byte).unwrap();
                        acc
                    });

            Ok(format!(
                "private_key (hex): {}\n\
                private_key (wif): {}\n\
                public_key (compressed): {}\n\
                address (compressed): {}\n\n",
                formatted_private_key_hex,
                res.get_wif_private_key(),
                res.get_comp_public_key(),
                res.get_comp_address()
            ))
        }
        Err(err) => Err(format!("Skipping because of error: {}\n\n", err)),
    }
}

/// Writes the result to an output file or prints to stdout
fn handle_output(string_flags: &CliFlags, buffer1: &str, buffer2: &str) {
    if !string_flags.output_file_name.is_empty() {
        write_output_file(
            &string_flags.output_file_name,
            &format!("{}\n{}", buffer1, buffer2),
        )
        .unwrap();
    } else {
        println!("{}", buffer2);
    }
}

fn main() {
    let cli_flags = parse_cli();

    for (i, string) in cli_flags.get_strings().iter().enumerate() {
        let string_flags = get_strings_flags(&cli_flags, i);

        let (vanity_mode_str, case_sensitive_str) = get_decoration_strings(
            string_flags.get_vanity_mode(),
            string_flags.get_case_sensitivity(),
        );

        print_initial_message(
            cli_flags.chain,
            cli_flags.threads,
            vanity_mode_str,
            string,
            case_sensitive_str,
        );

        let buffer1 = format!(
            "Key pair which their address {}: '{}' {}\n",
            vanity_mode_str, string, case_sensitive_str
        );

        let result = generate_vanity_address(string, cli_flags.threads, &cli_flags);

        match result {
            Ok(buffer2) => handle_output(&cli_flags, &buffer1, &buffer2),
            Err(error_message) => eprintln!("{}", error_message),
        }
    }
}
