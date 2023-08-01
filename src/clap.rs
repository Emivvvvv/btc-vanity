use clap;

fn cli() -> clap::Command {
    clap::Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::new("prefix")
                .index(1)
                .required(true)
                .help("Prefix used to match addresses"),
        )
        .arg(
            clap::Arg::new("threads")
                .short('t')
                .long("threads")
                .default_value("8")
                .help("Number of threads to be used"),
        )
        .arg(
            clap::Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .action(clap::ArgAction::SetTrue)
                .help("Use case sensitive comparison to match addresses"),
        )
        .arg(
            clap::Arg::new("disable-fast-mode")
                .short('d')
                .long("disable-fast")
                .action(clap::ArgAction::SetTrue)
                .help("Disables fast mode to find a prefix more than 4 characters"),
        )
}

pub fn args() -> (String, u64, bool, bool) {
    let app = cli();
    let matches = app.get_matches();

    return(
        matches.get_one::<String>("prefix")
            .expect("This was unexpected :(. Something went wrong while getting prefix arg")
            .to_string(),
        matches.get_one::<String>("threads")
            .expect("This was unexpected :(. Something went wrong while getting -t or --threads arg")
            .trim().parse::<u64>()
            .expect("Threads must be a number!"),
        matches.get_flag("case-sensitive"),
        matches.get_flag("disable-fast-mode"),
    )
}