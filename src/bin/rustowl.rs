//! # RustOwl cargo-owlsp
//!
//! An LSP server for visualizing ownership and lifetimes in Rust, designed for debugging and optimization.

use clap_complete::generate;
use rustowl::cli::cli;
use rustowl::shells::Shell;
use rustowl::*;
use std::env;
use std::io;
use tower_lsp::{LspService, Server};

fn set_log_level(default: log::LevelFilter) {
    log::set_max_level(
        env::var("RUST_LOG")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default),
    );
}

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .init()
        .unwrap();
    set_log_level("info".parse().unwrap());

    let matches = cli().get_matches();
    if let Some(arg) = matches.subcommand() {
        match arg {
            ("check", _) => {
                if Backend::check(env::current_dir().unwrap()).await {
                    log::info!("Successfully analyzed");
                    std::process::exit(0);
                } else {
                    log::error!("Analyze failed");
                    std::process::exit(1);
                }
            }
            ("clean", _) => {
                if let Ok(meta) = cargo_metadata::MetadataCommand::new().exec() {
                    let target = meta.target_directory.join("owl");
                    tokio::fs::remove_dir_all(&target).await.ok();
                }
            }
            ("toolchain", matches) => match matches.subcommand() {
                Some(("install", _)) => if rustowl::toolchain::setup_toolchain().await.is_err() {},
                Some(("uninstall", _)) => {
                    rustowl::toolchain::uninstall_toolchain().await;
                }
                _ => {}
            },
            ("completions", matches) => {
                set_log_level("off".parse().unwrap());
                let shell = matches
                    .get_one::<Shell>("shell")
                    .expect("shell is required by clap");
                generate(*shell, &mut cli(), "rustowl", &mut io::stdout());
            }
            _ => {}
        }
    } else if matches.get_flag("version") {
        if matches.get_count("quiet") == 0 {
            print!("RustOwl ");
        }
        println!("v{}", clap::crate_version!());
        return;
    } else {
        set_log_level("warn".parse().unwrap());
        eprintln!("RustOwl v{}", clap::crate_version!());
        eprintln!("This is an LSP server. You can use --help flag to show help.");

        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::build(Backend::new)
            .custom_method("rustowl/cursor", Backend::cursor)
            .finish();
        Server::new(stdin, stdout, socket).serve(service).await;
    }
}
