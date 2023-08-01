use clap;

pub fn cli() -> clap::Command {
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
            clap::Arg::new("fast_mode")
                .conflicts_with("disable_fast_mode")
                .short('f')
                .long("fast")
                .action(clap::ArgAction::SetTrue)
                .help("Limits the prefix with 4 characters"),
        )
        .arg(
            clap::Arg::new("disable_fast_mode")
                .short('d')
                .long("disable-fast")
                .action(clap::ArgAction::SetFalse)
                .help("Disables fast mode to find a prefix more than 4 characters"),
        )
}