use std::time::Instant;
use btc_vanity::vanity_addr_generator::VanityAddr;
use btc_vanity::clap::{cli};
use btc_vanity::file::write_output_file;
use btc_vanity::decoration::get_decoration_strings;
use btc_vanity::flags_and_args::{get_cli_args, get_strings_args};

fn main() {
    // Sets the cli app.
    let app = cli();

    let cli_args = get_cli_args(app);

    // Loop for multiple wallet inputs from text file.
    for (i, string) in cli_args.get_strings().iter().enumerate() {
        let string_args = get_strings_args(&cli_args, i);


        let (vanity_mode_str, case_sensitive_str)
            = get_decoration_strings(
            string_args.get_vanity_mode(),
            string_args.get_case_sensitivity());

        // First buffer/print before starting calculation
        let mut buffer1 = String::new();
        println!("Searching key pair which their address {}: '{}' {} with {} threads.\n",
                 vanity_mode_str,
                 string,
                 case_sensitive_str,
                 cli_args.get_threads());
        if !string_args.get_output_file_name().is_empty() { buffer1 = format!("Key pair which their address {}: '{}' {}\n",
                                                            vanity_mode_str,
                                                            string,
                                                            case_sensitive_str) }

        // Generates the vanity address and measures the time elapsed while finding the address.
        let start = Instant::now();
        let result = VanityAddr::generate(
            string,
            cli_args.get_threads(),
            string_args.get_case_sensitivity(),
            !string_args.get_is_fast_mode_disabled(),
            string_args.get_vanity_mode());
        let seconds = start.elapsed().as_secs_f64();

        // Second buffer/print after the vanity address found
        #[allow(unused_assignments)]
        let mut buffer2 = String::new();
        match result{
            Ok(res) => {
                println!("FOUND IN {:.4} SECONDS!\n", seconds);
                // Prints the found key pair and the address which has the string.
                buffer2 = format!(
                    "private_key (wif): {}\n\
                    public_key (compressed): {}\n\
                    address (compressed): {}\n\n",
                        res.get_wif_private_key(),
                        res.get_comp_public_key(),
                        res.get_comp_address())
            },
            Err(err) => buffer2 = format!("Skipping because of error: {}\n\n", err),
        }

        // If string_output_file_name is empty it just prints the buffer2 to stdout else writes the wallet to the output file.
        if !string_args.get_output_file_name().is_empty() { write_output_file(string_args.get_output_file_name(), &format!("{}\n{}", buffer1, buffer2)).unwrap() }
        else {println!("{}", buffer2)}
    }
}