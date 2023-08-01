use btc_vanity::vanity_addr_generator::VanityAddr;
use btc_vanity::clap::args;

fn main() {
    // Sets the cli app and gets all the arguments from cli app
    let (prefix, threads, is_case_sensitive, is_fast_disabled) = args();

    if !is_case_sensitive {panic!("for now it just work with case-sensitive mode. use -c flag to generate a vanity address.")}

    // Gets the keys and the vanity address
    println!("Searching key pair which their address has the prefix: '{}' with {} threads", prefix, threads);
    let result = VanityAddr::generate(prefix, threads, is_case_sensitive, !is_fast_disabled).unwrap();

    println!("FOUND!\n\
    private_key (wif): {}\n\
    public_key (compressed): {}\n\
    address (compressed): {}",
             result.get_wif_private_key(),
             result.get_comp_public_key(),
             result.get_comp_address())
}