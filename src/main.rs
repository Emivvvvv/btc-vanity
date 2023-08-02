use std::time::Instant;
use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
use btc_vanity::clap::{cli};
use std::fs;

const MODE: [&str; 3] = ["has the prefix", "has the suffix", "has the string"];
const CASE_SENSITIVITY: [&str; 2] = ["(case sensitive)", "(case sensitivity disabled)"];

fn get_strings_from_file(file_name: &String) -> Result<Vec<String>, Box<dyn std::error::Error>>{
    let data = fs::read_to_string(file_name)?;
    let lines: Vec<&str> = data.lines().collect::<Vec<_>>();
    let strings = lines.into_iter().map(|line| line.to_string()).collect();
    Ok(strings)
}

fn main() {
    // Sets the cli app
    let app = cli();

    // Gets all the arguments from the cli
    let matches = app.get_matches();
    let threads = matches.get_one::<String>("threads")
        .expect("This was unexpected :(. Something went wrong while getting -t or --threads arg")
        .trim().parse::<u64>()
        .expect("Threads must be a number!");
    let is_case_sensitive = matches.get_flag("case-sensitive");
    let is_fast_disabled = matches.get_flag("disable-fast-mode");
    let strings = match matches.get_one::<String>("string") {
        Some(string) => vec![string.to_owned()],
        None => {
            let file_name = matches.get_one::<String>("input-file").unwrap();
            get_strings_from_file(file_name).unwrap()
        }
    };

    // Sets vanity_mode for searching and mode to predefined decoration strings.
    let (vanity_mode, mode) =
        if matches.get_flag("anywhere") { (VanityMode::Anywhere, MODE[2]) }
        else if matches.get_flag("suffix") { (VanityMode::Suffix, MODE[1]) }
        else { (VanityMode::Anywhere, MODE[0]) };

    // Sets case sensitivity decoration string.
    let case_sensitive = match is_case_sensitive {
        true => CASE_SENSITIVITY[0],
        false => CASE_SENSITIVITY[1],
    };

    for string in strings {
        // Gets the keys and the vanity address.
        println!("Searching key pair which their address {}: '{}' {} with {} threads.\n",
                 mode,
                 string,
                 case_sensitive,
                 threads);

        // Generates the vanity address and measures the time elapsed while finding the address.
        let start = Instant::now();
        let result = VanityAddr::generate(
            string,
            threads,
            is_case_sensitive,
            !is_fast_disabled,
            vanity_mode);
        let seconds = start.elapsed().as_secs_f64();

        match result{
            Ok(res) => {
                // Prints the found key pair and the address which has the string.
                println!(
                    "FOUND IN {:.4} SECONDS!\n\
                    private_key (wif): {}\n\
                    public_key (compressed): {}\n\
                    address (compressed): {}\n",
                        seconds,
                        res.get_wif_private_key(),
                        res.get_comp_public_key(),
                        res.get_comp_address())
            },
            Err(err) => println!("Skipping because of error: {}\n", err),
        }
    }
}