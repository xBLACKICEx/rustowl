use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

fn main() {
    simple_logger::init().unwrap();

    let self_path = PathBuf::from(env::args().nth(0).unwrap());
    let root_dir = PathBuf::from(env::args().nth(2).unwrap_or(".".to_owned()));
    let target_dir = PathBuf::from(env::args().nth(3).unwrap_or("./target".to_owned()));
    let mut command = Command::new("rustup");

    #[cfg(target_os = "windows")]
    command.env(
        "Path",
        format!(
            "{}{}",
            env::var("LD_LIBRARY_PATH")
                .map(|path| format!("{path}\\..\\bin;"))
                .unwrap_or("".to_owned()),
            env::var("Path").unwrap_or("".to_owned())
        ),
    );

    command
        .env(
            "RUSTC_WORKSPACE_WRAPPER",
            self_path.with_file_name("rustowlc"),
        )
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTC_WRAPPER")
        .arg("run")
        .arg("nightly-2024-10-31")
        .arg("cargo")
        .arg("check")
        .current_dir(&root_dir);

    let code = command.spawn().unwrap().wait().unwrap().code().unwrap();
    exit(code);
}
