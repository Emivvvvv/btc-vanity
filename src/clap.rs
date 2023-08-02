use clap;

pub fn cli() -> clap::Command {
    clap::Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::new("string")
                .index(1)
                .required_unless_present_any(["input-file"])
                .help("String used to match addresses."),
        )
        .arg(
            clap::Arg::new("input-file")
                .short('i')
                .long("input-file")
                .required_unless_present_any(["string"])
                .help("File with strings to match addresses with.\nImportant: Write every string in a separate line.")
        )
        .arg(
            clap::Arg::new("force-flags")
                .short('f')
                .long("force-flags")
                .action(clap::ArgAction::SetTrue)
                .help("Use this flag to override the flags in the input file\nor use in file to override cli flags for only that string.\nNote: Cli -f is stronger than input-file -f.")
        )
        .arg(
            clap::Arg::new("output-file")
                .short('o')
                .long("output-file")
                .help("Crates a file that contains found wallet/s."),
        )
        .arg(
            clap::Arg::new("prefix")
                .conflicts_with("suffix")
                .conflicts_with("anywhere")
                .short('p')
                .long("prefix")
                .action(clap::ArgAction::SetTrue)
                .help("Finds a vanity address which has 'string' prefix. [default]")
        )
        .arg(
            clap::Arg::new("suffix")
                .conflicts_with("prefix")
                .conflicts_with("anywhere")
                .short('s')
                .long("suffix")
                .action(clap::ArgAction::SetTrue)
                .help("Finds a vanity address which has 'string' suffix.")
        )
        .arg(
            clap::Arg::new("anywhere")
                .short('a')
                .long("anywhere")
                .action(clap::ArgAction::SetTrue)
                .help("Finds a vanity address which includes 'string' at any part of the address.")
        )
        .arg(
            clap::Arg::new("threads")
                .short('t')
                .long("threads")
                .default_value("16")
                .help("Number of threads to be used."),
        )
        .arg(
            clap::Arg::new("case-sensitive")
                .short('c')
                .long("case-sensitive")
                .action(clap::ArgAction::SetTrue)
                .help("Use case sensitive comparison to match addresses."),
        )
        .arg(
            clap::Arg::new("disable-fast-mode")
                .short('d')
                .long("disable-fast")
                .action(clap::ArgAction::SetTrue)
                .help("Disables fast mode to find a prefix more than 4 characters."),
        )
}