use btc_vanity::cli::cli;
use btc_vanity::error::VanityError;
use btc_vanity::file::{parse_input_file, write_output_file};
use btc_vanity::flags::{parse_cli, PatternsSource, VanityFlags};
use btc_vanity::keys_and_address::BitcoinKeyPair;
use btc_vanity::vanity_addr_generator::chain::Chain;
use btc_vanity::vanity_addr_generator::vanity_addr::{VanityAddr, VanityMode};
#[cfg(feature = "ethereum")]
use btc_vanity::EthereumKeyPair;
#[cfg(any(feature = "ethereum", feature = "solana"))]
use btc_vanity::KeyPairGenerator;
#[cfg(feature = "solana")]
use btc_vanity::SolanaKeyPair;

#[cfg(feature = "solana")]
use bitcoin::hex::DisplayHex;
use clap::error::ErrorKind;
use std::path::Path;
use std::process;
use std::time::Instant;

/// Generates and formats a vanity address depending on the chain.
/// Returns a `Result<String, String>` where the `Ok(String)` is the final formatted output.
fn generate_vanity_address(pattern: &str, vanity_flags: &VanityFlags) -> Result<String, String> {
    let start = Instant::now();

    // "Inline" everything in each arm so we get a single `Result<String, String>`
    let out = match vanity_flags.chain.unwrap_or(Chain::Bitcoin) {
        Chain::Bitcoin => {
            // 1) Generate the Bitcoin vanity
            let result: Result<BitcoinKeyPair, VanityError> =
                match vanity_flags.vanity_mode.unwrap_or(VanityMode::Prefix) {
                    VanityMode::Regex => {
                        VanityAddr::generate_regex::<BitcoinKeyPair>(pattern, vanity_flags.threads)
                    }
                    _ => VanityAddr::generate::<BitcoinKeyPair>(
                        pattern,
                        vanity_flags.threads,
                        vanity_flags.is_case_sensitive,
                        !vanity_flags.disable_fast_mode,
                        vanity_flags.vanity_mode.unwrap_or(VanityMode::Prefix),
                    ),
                };

            // 2) Format the result on success
            match result {
                Ok(res) => {
                    let s = format!(
                        "private_key (wif): {}\n\
                         public_key (compressed): {}\n\
                         address (compressed): {}\n\n",
                        res.get_wif_private_key(),
                        res.get_comp_public_key(),
                        res.get_comp_address()
                    );
                    Ok(s)
                }
                Err(e) => Err(e.to_string()),
            }
        }

        #[cfg(feature = "ethereum")]
        Chain::Ethereum => {
            // 1) Generate the Ethereum vanity
            let result: Result<EthereumKeyPair, VanityError> =
                match vanity_flags.vanity_mode.unwrap_or(VanityMode::Prefix) {
                    VanityMode::Regex => {
                        VanityAddr::generate_regex::<EthereumKeyPair>(pattern, vanity_flags.threads)
                    }
                    _ => VanityAddr::generate::<EthereumKeyPair>(
                        pattern,
                        vanity_flags.threads,
                        vanity_flags.is_case_sensitive,
                        !vanity_flags.disable_fast_mode,
                        vanity_flags.vanity_mode.unwrap_or(VanityMode::Prefix),
                    ),
                };

            // 2) Format on success
            match result {
                Ok(res) => {
                    let private_key_hex = res.get_private_key_as_hex();
                    let pub_key_hex = res.get_public_key_as_hex();
                    let address = res.get_address();

                    let s = format!(
                        "private_key (hex): 0x{private_key_hex}\n\
                         public_key (hex): 0x{pub_key_hex}\n\
                         address: 0x{address}\n\n"
                    );
                    Ok(s)
                }
                Err(e) => Err(e.to_string()),
            }
        }

        #[cfg(feature = "solana")]
        Chain::Solana => {
            // 1) Generate the Solana vanity
            let result: Result<SolanaKeyPair, VanityError> =
                match vanity_flags.vanity_mode.unwrap_or(VanityMode::Prefix) {
                    VanityMode::Regex => {
                        VanityAddr::generate_regex::<SolanaKeyPair>(pattern, vanity_flags.threads)
                    }
                    _ => VanityAddr::generate::<SolanaKeyPair>(
                        pattern,
                        vanity_flags.threads,
                        vanity_flags.is_case_sensitive,
                        !vanity_flags.disable_fast_mode,
                        vanity_flags.vanity_mode.unwrap_or(VanityMode::Prefix),
                    ),
                };

            // 2) Format on success
            match result {
                Ok(res) => {
                    // Keypair -> hex
                    let keypair_bytes = res.keypair().to_bytes();
                    let private_key_hex = keypair_bytes.as_hex();

                    let address = res.get_address();
                    let s = format!(
                        "private_key (hex): {private_key_hex}\n\
                         address: {address}\n\n"
                    );
                    Ok(s)
                }
                Err(e) => Err(e.to_string()),
            }
        }
        // This arm handles missing features.
        #[cfg(not(feature = "ethereum"))]
        Chain::Ethereum => Err(VanityError::MissingFeatureEthereum.to_string()),
        #[cfg(not(feature = "solana"))]
        Chain::Solana => Err(VanityError::MissingFeatureSolana.to_string()),
    };

    // If we made it here, we have out: Result<String, String>
    match out {
        Ok(s) => {
            let seconds = start.elapsed().as_secs_f64();
            println!("FOUND IN {seconds:.4} SECONDS!\n");
            Ok(s)
        }
        Err(e) => Err(format!("Skipping because of error: {e}\n\n")),
    }
}

/// A single function to handle generating and printing/writing a vanity address
/// for a given `pattern` + final `VanityFlags`.
fn handle_item(pattern: &str, flags: &VanityFlags) {
    // 1) Some fancy text decorations
    // If `flags.vanity_mode` is None, we default to `VanityMode::Prefix`; up to you
    let vanity_mode = flags.vanity_mode.unwrap_or(VanityMode::Prefix);

    // For the "case_sensitive_str":
    let case_str = if flags.is_case_sensitive {
        "(case sensitive)"
    } else {
        "(case insensitive)"
    };

    // Possibly you have a function like get_decoration_strings(...)
    // We can replicate it inline:
    let (vanity_mode_str, _case_sensitive_str) = match vanity_mode {
        VanityMode::Prefix => ("has the prefix", case_str),
        VanityMode::Suffix => ("has the suffix", case_str),
        VanityMode::Anywhere => ("has the string", case_str),
        VanityMode::Regex => ("matches regex", case_str),
    };

    // 2) Print the "initial search message"
    // If chain is None, default to Bitcoin, or handle differently:
    let chain = flags.chain.unwrap_or(Chain::Bitcoin);
    println!(
        "Searching key pair for {:?} chain where the address {}: '{}' {} with {} threads.\n",
        chain, vanity_mode_str, pattern, case_str, flags.threads
    );

    // 3) Build "buffer1"
    let buffer1 = format!("Key pair whose address {vanity_mode_str}: '{pattern}' {case_str}\n");

    // 4) Actually generate the address
    let result = generate_vanity_address(pattern, flags);

    // 5) Format result or error, then handle output
    match result {
        Ok(buffer2) => {
            // If the user gave an output_file_name, write to that file.
            // Otherwise, print to stdout.
            if let Some(ref file_path) = flags.output_file_name {
                // example from your existing code:
                if let Err(e) =
                    write_output_file(Path::new(file_path), &format!("{buffer1}\n{buffer2}"))
                {
                    eprintln!("Failed to write output: {e}");
                }
            } else {
                println!("{buffer2}");
            }
        }
        Err(error_message) => {
            eprintln!("{error_message}");
        }
    }
}

fn main() {
    let app = cli();
    let (cli_flags, source) = match app.try_get_matches() {
        Ok(matches) => parse_cli(matches),
        Err(err) => {
            if err.kind() == ErrorKind::MissingRequiredArgument {
                eprintln!(
                    "error: the following required arguments were not provided:\n  --input-file <input-file> OR <string>\n"
                );
                eprintln!("Usage: btc-vanity [OPTIONS] <string>");
            } else {
                eprintln!("{err}");
            }
            process::exit(1);
        }
    };

    // 4) Decide how we get our pattern(s):
    match source {
        PatternsSource::SingleString(pattern) => {
            // We only have one pattern. "Unify" is trivial because there's no file-based flags
            // So just use our CLI flags directly
            handle_item(&pattern, &cli_flags);
        }

        PatternsSource::InputFile(file_path) => {
            // The user specified an input file, so parse each line
            let items = match parse_input_file(&file_path) {
                Ok(lines) => lines,
                Err(e) => {
                    eprintln!("Error reading file '{file_path}': {e}");
                    process::exit(1);
                }
            };

            // For each line in the file, unify that line’s flags with the CLI’s flags
            for file_item in items {
                // Merge the line flags + CLI flags
                let final_flags = cli_flags.unify(&file_item.flags);
                // Then handle the pattern from that line
                handle_item(&file_item.pattern, &final_flags);
            }
        }
    }
}
