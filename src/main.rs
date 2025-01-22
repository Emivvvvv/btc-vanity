use btc_vanity::cli::cli;
use btc_vanity::decoration::get_decoration_strings;
use btc_vanity::file::write_output_file;
use btc_vanity::flags::{get_cli_flags, get_strings_flags, Chain, CliFlags};
use btc_vanity::keys_and_address::{BitcoinKeyPair, EthereumKeyPair, SolanaKeyPair};
use btc_vanity::vanity_addr_generator::vanity_addr::{VanityAddr, VanityMode};
use btc_vanity::error::VanityError;

use clap::error::ErrorKind;
use std::fmt::Write as FmtWrite;
use std::time::Instant;
use btc_vanity::KeyPairGenerator;

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
        "Searching key pair for {:?} chain where the address {}: '{}' {} with {} threads.\n",
        chain, vanity_mode_str, string, case_sensitive_str, threads
    );
}

/// Generates and formats a vanity address depending on the chain.
/// Returns a `Result<String, String>` where the `Ok(String)` is the final formatted output.
fn generate_vanity_address(
    pattern: &str,
    threads: u64,
    cli_flags: &CliFlags,
) -> Result<String, String> {
    let start = Instant::now();

    // "Inline" everything in each arm so we get a single `Result<String, String>`
    let out = match cli_flags.chain {
        Chain::Bitcoin => {
            // 1) Generate the Bitcoin vanity
            let result: Result<BitcoinKeyPair, VanityError> = match cli_flags.vanity_mode {
                VanityMode::Regex => {
                    VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, threads)
                }
                _ => VanityAddr::generate::<BitcoinKeyPair>(
                    pattern,
                    threads,
                    cli_flags.is_case_sensitive,
                    !cli_flags.is_fast_disabled,
                    cli_flags.vanity_mode,
                ),
            };

            // 2) Format the result on success
            match result {
                Ok(res) => {
                    // Convert to hex
                    let private_key_hex = res
                        .get_private_key()
                        .to_bytes()
                        .iter()
                        .fold(String::new(), |mut acc: String, &byte: &u8| {
                            write!(&mut acc, "{:02X}", byte).unwrap();
                            acc
                        });

                    // Build the final output string
                    let s = format!(
                        "private_key (hex): {}\n\
                         private_key (wif): {}\n\
                         public_key (compressed): {}\n\
                         address (compressed): {}\n\n",
                        private_key_hex,
                        res.get_wif_private_key(),
                        res.get_comp_public_key(),
                        res.get_comp_address()
                    );
                    Ok(s)
                }
                Err(e) => Err(e.to_string()),
            }
        }

        Chain::Ethereum => {
            // 1) Generate the Ethereum vanity
            let result: Result<EthereumKeyPair, VanityError> = match cli_flags.vanity_mode {
                VanityMode::Regex => {
                    VanityAddr::generate_regex::<EthereumKeyPair>(pattern, threads)
                }
                _ => VanityAddr::generate::<EthereumKeyPair>(
                    pattern,
                    threads,
                    cli_flags.is_case_sensitive,
                    !cli_flags.is_fast_disabled,
                    cli_flags.vanity_mode,
                ),
            };

            // 2) Format on success
            match result {
                Ok(res) => {
                    // Convert private key to hex
                    let private_key_hex = res
                        .private_key
                        .as_ref()
                        .iter()
                        .fold(String::new(), |mut acc: String, &byte: &u8| {
                            write!(&mut acc, "{:02X}", byte).unwrap();
                            acc
                        });

                    // Convert uncompressed pubkey to hex
                    let pub_uncompressed = res.public_key.serialize_uncompressed();
                    let pub_hex_str = pub_uncompressed[1..].iter().fold(
                        String::new(),
                        |mut acc: String, &byte: &u8| {
                            write!(&mut acc, "{:02X}", byte).unwrap();
                            acc
                        },
                    );

                    let address = res.get_address();

                    let s = format!(
                        "private_key (hex): 0x{}\n\
                         public_key (uncompressed): 0x{}\n\
                         address: 0x{}\n\n",
                        private_key_hex, pub_hex_str, address
                    );
                    Ok(s)
                }
                Err(e) => Err(e.to_string()),
            }
        }

        Chain::Solana => {
            // 1) Generate the Solana vanity
            let result: Result<SolanaKeyPair, VanityError> = match cli_flags.vanity_mode {
                VanityMode::Regex => {
                    VanityAddr::generate_regex::<SolanaKeyPair>(pattern, threads)
                }
                _ => VanityAddr::generate::<SolanaKeyPair>(
                    pattern,
                    threads,
                    cli_flags.is_case_sensitive,
                    !cli_flags.is_fast_disabled,
                    cli_flags.vanity_mode,
                ),
            };

            // 2) Format on success
            match result {
                Ok(res) => {
                    // Keypair -> hex
                    let keypair_bytes = res.keypair.to_bytes();
                    let private_key_hex = keypair_bytes.iter().fold(
                        String::new(),
                        |mut acc: String, &byte: &u8| {
                            write!(&mut acc, "{:02X}", byte).unwrap();
                            acc
                        },
                    );

                    let address = res.get_address();
                    let s = format!(
                        "private_key (hex): {}\n\
                         address: {}\n\n",
                        private_key_hex, address
                    );
                    Ok(s)
                }
                Err(e) => Err(e.to_string()),
            }
        }
    };

    // If we made it here, we have out: Result<String, String>
    match out {
        Ok(s) => {
            let seconds = start.elapsed().as_secs_f64();
            println!("FOUND IN {:.4} SECONDS!\n", seconds);
            Ok(s)
        }
        Err(e) => Err(format!("Skipping because of error: {}\n\n", e)),
    }
}

/// Writes the result to an output file or prints to stdout
fn handle_output(cli_flags: &CliFlags, buffer1: &str, buffer2: &str) {
    if !cli_flags.output_file_name.is_empty() {
        write_output_file(
            &cli_flags.output_file_name,
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
            "Key pair whose address {}: '{}' {}\n",
            vanity_mode_str, string, case_sensitive_str
        );

        let result = generate_vanity_address(string, cli_flags.threads, &cli_flags);

        match result {
            Ok(buffer2) => handle_output(&cli_flags, &buffer1, &buffer2),
            Err(error_message) => eprintln!("{}", error_message),
        }
    }
}
