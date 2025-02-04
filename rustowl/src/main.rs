mod toolchain_version;

use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

use toolchain_version::TOOLCHAIN_VERSION;

fn main() {
    simple_logger::init().unwrap();

    let self_path = PathBuf::from(env::args().nth(0).unwrap());
    let root_dir = PathBuf::from(env::args().nth(2).unwrap_or(".".to_owned()));
    let target_dir = PathBuf::from(env::args().nth(3).unwrap_or("./target".to_owned()));

    #[cfg(windows)]
    unsafe {
        use std::ffi::OsString;
        let triple_suffix = env::var("RUSTUP_TOOLCHAIN")
            .unwrap()
            .split("-")
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(4)
            .rev()
            .collect::<Vec<_>>()
            .join("-");
        let value = OsString::from(format!(
            "{}{}",
            env::var("RUSTUP_HOME")
                .map(|path| format!(
                    "{path}\\toolchains\\{TOOLCHAIN_VERSION}-{triple_suffix}\\bin;"
                ))
                .unwrap_or("".to_owned()),
            env::var("Path").unwrap_or("".to_owned()),
        ));
        env::set_var("Path", &value);
    }

    let mut command = Command::new("cargo");
    command
        .env(
            "RUSTC_WORKSPACE_WRAPPER",
            self_path.with_file_name("rustowlc"),
        )
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTC_WRAPPER")
        .arg("check")
        .current_dir(&root_dir);

    let code = command.spawn().unwrap().wait().unwrap().code().unwrap();
    exit(code);
}
