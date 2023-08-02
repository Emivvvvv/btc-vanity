use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
use btc_vanity::clap::args;

const MODE: [&str; 3] = ["has the prefix", "has the suffix", "has the string"];

fn main() {
    // Sets the cli app and gets all the arguments from cli app.
    let (string, threads, is_case_sensitive, is_fast_disabled, vanity_mode) = args();

    // Panics if -c flag is not set. case_sensitive: false mode is not implemented yet.
    if !is_case_sensitive {panic!("for now it just work with case-sensitive mode. use -c flag to generate a vanity address.")}

    // Sets mode to predefined decoration strings.
    let mode = match vanity_mode {
        VanityMode::Prefix => MODE[0],
        VanityMode::Suffix => MODE[1],
        VanityMode::Anywhere => MODE[2],
    };

    // Gets the keys and the vanity address
    println!("Searching key pair which their address {}: '{}' with {} threads.\n", mode, string, threads);
    let result = VanityAddr::generate(string, threads, is_case_sensitive, !is_fast_disabled, vanity_mode).unwrap();

    // Prints the found key pair and the address which has the string.
    println!("FOUND!\n\
    private_key (wif): {}\n\
    public_key (compressed): {}\n\
    address (compressed): {}",
             result.get_wif_private_key(),
             result.get_comp_public_key(),
             result.get_comp_address())
}