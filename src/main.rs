use std::time::Instant;
use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
use btc_vanity::clap::{cli};
use btc_vanity::file::{FileFlags, get_strings_and_flags_from_file, write_output_file};

const VANITY_MODE_STR: [&str; 3] = ["has the prefix", "has the suffix", "has the string"];
const CASE_SENSITIVITY_STR: [&str; 2] = ["(case sensitive)", "(case sensitivity disabled)"];

fn main() {
    // Sets the cli app.
    let app = cli();

    // Gets all the arguments from the cli.
    let matches = app.get_matches();
    let threads = matches.get_one::<String>("threads")
        .expect("This was unexpected :(. Something went wrong while getting -t or --threads arg")
        .trim().parse::<u64>()
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
    let cli_vanity_mode =
        if matches.get_flag("anywhere") { VanityMode::Anywhere }
        else if matches.get_flag("suffix") { VanityMode::Suffix }
        else { VanityMode::Prefix };

    let mut string_is_case_sensitive;
    let mut string_is_fast_disabled;
    let mut string_output_file_name;
    let mut string_vanity_mode;

    // Loop for multiple wallet inputs from text file.
    for (i, string) in strings.iter().enumerate() {
        if cli_force_flags {
            string_is_case_sensitive = cli_is_case_sensitive;
            string_is_fast_disabled = cli_is_fast_disabled;
            string_output_file_name = &cli_output_file_name;
            string_vanity_mode = cli_vanity_mode;
        } else {
            let flags = &flags_vec[i];
            let force_flags = flags.force_flags;
            string_vanity_mode = match flags.vanity_mode {
                Some(vanity_mode) => vanity_mode,
                None => cli_vanity_mode
            };
            string_output_file_name = match &flags.output_file_name {
                Some(output_file_name) => output_file_name,
                None => &cli_output_file_name,
            };
            if force_flags {
                string_is_case_sensitive = flags.is_case_sensitive;
                string_is_fast_disabled = flags.disable_fast_mode;
            } else {
                string_is_case_sensitive = cli_is_case_sensitive || flags.is_case_sensitive;
                string_is_fast_disabled = cli_is_fast_disabled || flags.is_case_sensitive;
            }
        }

        let vanity_mode_str = match string_vanity_mode {
            VanityMode::Prefix => VANITY_MODE_STR[0],
            VanityMode::Suffix => VANITY_MODE_STR[1],
            VanityMode::Anywhere => VANITY_MODE_STR[2],
        };

        // Sets case sensitivity decoration string.
        let case_sensitive_str = match string_is_case_sensitive {
            true => CASE_SENSITIVITY_STR[0],
            false => CASE_SENSITIVITY_STR[1],
        };

        let mut buffer1 = String::new();
        println!("Searching key pair which their address {}: '{}' {} with {} threads.\n",
                 vanity_mode_str,
                 string,
                 case_sensitive_str,
                 threads);

        if !string_output_file_name.is_empty() { buffer1 = format!("Key pair which their address {}: '{}' {}\n",
                                                            vanity_mode_str,
                                                            string,
                                                            case_sensitive_str) }

        // Generates the vanity address and measures the time elapsed while finding the address.
        let start = Instant::now();
        let result = VanityAddr::generate(
            string,
            threads,
            string_is_case_sensitive,
            !string_is_fast_disabled,
            string_vanity_mode);
        let seconds = start.elapsed().as_secs_f64();

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

        if !string_output_file_name.is_empty() { write_output_file(&string_output_file_name, &format!("{}\n{}", buffer1, buffer2)).unwrap() }
        else {println!("{}", buffer2)}
    }
}