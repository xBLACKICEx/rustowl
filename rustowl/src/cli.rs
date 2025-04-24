pub fn cli() -> clap::Command {
    clap::Command::new("RustOwl Language Server")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .arg(
            clap::Arg::new("io")
                .long("stdio")
                .required(false)
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand_required(false)
        .subcommand(
            clap::Command::new("check").arg(
                clap::Arg::new("log_level")
                    .long("log")
                    .required(false)
                    .action(clap::ArgAction::Set),
            ),
        )
        .subcommand(clap::Command::new("clean"))
        .subcommand(clap::Command::new("toolchain").subcommand(clap::Command::new("uninstall")))
        .subcommand(
            clap::Command::new("completions")
            .about("Generate shell completions")
            .arg(
                clap::Arg::new("shell")
                .help("The shell to generate completions for")
                .required(true)
                .value_parser(clap::value_parser!(crate::shells::Shell))
            )
                                                                )
}
