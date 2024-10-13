use btc_vanity::cli::cli;
use btc_vanity::decoration::get_decoration_strings;
use btc_vanity::file::write_output_file;
use btc_vanity::flags::{get_cli_flags, get_strings_flags};
use btc_vanity::vanity_addr_generator::VanityAddr;
use clap::error::ErrorKind;
use std::fmt::Write;
use std::time::Instant;

fn main() {
    // Sets the cli app.
    let app = cli();

    // Try to parse the arguments and catch errors
    let cli_flags = match app.try_get_matches() {
        Ok(matches) => get_cli_flags(matches),
        Err(err) => {
            // Check if it's a missing argument error
            if err.kind() == ErrorKind::MissingRequiredArgument {
                // Customize the error message
                eprintln!(
                    "error: the following required arguments were not provided:\n  --input-file <input-file> OR <string>\n"
                );
                // Optionally, show the help message
                eprintln!("Usage: btc-vanity [OPTIONS] <string>");
            } else {
                // Otherwise, print the default error
                eprintln!("{}", err);
            }
            std::process::exit(1); // Exit with an error code
        }
    };

    // Loop for multiple wallet inputs from text file.
    for (i, string) in cli_flags.get_strings().iter().enumerate() {
        let string_flags = get_strings_flags(&cli_flags, i);

        let (vanity_mode_str, case_sensitive_str) = get_decoration_strings(
            string_flags.get_vanity_mode(),
            string_flags.get_case_sensitivity(),
        );

        // First buffer/print before starting calculation
        let mut buffer1 = String::new();
        println!(
            "Searching key pair which their address {}: '{}' {} with {} threads.\n",
            vanity_mode_str,
            string,
            case_sensitive_str,
            cli_flags.get_threads()
        );
        if !string_flags.get_output_file_name().is_empty() {
            buffer1 = format!(
                "Key pair which their address {}: '{}' {}\n",
                vanity_mode_str, string, case_sensitive_str
            )
        }

        // Generates the vanity address and measures the time elapsed while finding the address.
        let start = Instant::now();
        let result = VanityAddr::generate(
            string,
            cli_flags.get_threads(),
            string_flags.get_case_sensitivity(),
            !string_flags.get_is_fast_mode_disabled(),
            string_flags.get_vanity_mode(),
        );
        let seconds = start.elapsed().as_secs_f64();

        // Second buffer/print after the vanity address found
        let buffer2 = match result {
            Ok(res) => {
                println!("FOUND IN {:.4} SECONDS!\n", seconds);

                // Format the private key hex value
                let formatted_private_key_hex =
                    res.get_private_key()
                        .to_bytes()
                        .iter()
                        .fold(String::new(), |mut acc, byte| {
                            write!(&mut acc, "{:02X}", byte).unwrap();
                            acc
                        });

                // Prints the found key pair and the address which has the string.
                format!(
                    "private_key (hex): {}\n\
                    private_key (wif): {}\n\
                    public_key (compressed): {}\n\
                    address (compressed): {}\n\n",
                    formatted_private_key_hex,
                    res.get_wif_private_key(),
                    res.get_comp_public_key(),
                    res.get_comp_address()
                )
            }
            Err(err) => format!("Skipping because of error: {}\n\n", err),
        };

        // If string_output_file_name is empty it just prints the buffer2 to stdout else writes the wallet to the output file.
        if !string_flags.get_output_file_name().is_empty() {
            write_output_file(
                string_flags.get_output_file_name(),
                &format!("{}\n{}", buffer1, buffer2),
            )
            .unwrap()
        } else {
            println!("{}", buffer2)
        }
    }
}
