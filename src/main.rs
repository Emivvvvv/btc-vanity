use btc_vanity::vanity_addr_generator::VanityAddr;
use btc_vanity::clap::cli;

fn main() {
    let app = cli();
    let matches = app.get_matches();
}