use std::time::Instant;
use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
use btc_vanity::clap::args;

const MODE: [&str; 3] = ["has the prefix", "has the suffix", "has the string"];
const CASE_SENSITIVITY: [&str; 2] = ["(case sensitive)", "(case sensitivity disabled)"];

fn main() {
    // Sets the cli app and gets all the arguments from cli app.
    let (string, threads, is_case_sensitive, is_fast_disabled, vanity_mode) = args();

    // Sets mode to predefined decoration strings.
    let mode = match vanity_mode {
        VanityMode::Prefix => MODE[0],
        VanityMode::Suffix => MODE[1],
        VanityMode::Anywhere => MODE[2],
    };

    // Sets case sensitivity decoration string.
    let case_sensitive = match is_case_sensitive {
        true => CASE_SENSITIVITY[0],
        false => CASE_SENSITIVITY[1],
    };

    // Gets the keys and the vanity address
    println!("Searching key pair which their address {}: '{}' {} with {} threads.\n", mode, string, case_sensitive, threads);
    let start = Instant::now();
    let result = VanityAddr::generate(string, threads, is_case_sensitive, !is_fast_disabled, vanity_mode).unwrap();
    let seconds = start.elapsed().as_secs_f64();

    // Prints the found key pair and the address which has the string.
    println!("FOUND IN {:.4} SECONDS!\n\
    private_key (wif): {}\n\
    public_key (compressed): {}\n\
    address (compressed): {}",
             seconds,
             result.get_wif_private_key(),
             result.get_comp_public_key(),
             result.get_comp_address())
}