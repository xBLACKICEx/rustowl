use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

fn main() {
    simple_logger::init().unwrap();

    let self_path = PathBuf::from(env::args().nth(0).unwrap());
    let root_dir = PathBuf::from(env::args().nth(2).unwrap_or(".".to_owned()));
    let target_dir = PathBuf::from(env::args().nth(3).unwrap_or("./target".to_owned()));

    #[cfg(windows)]
    unsafe {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::System::Environment::SetEnvironmentVariableW;
        let path: Vec<u16> = OsString::from("Path")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let value: Vec<u16> = OsString::from(format!(
            "{}{}",
            env::var("LD_LIBRARY_PATH")
                .map(|paths| paths
                    .split(";")
                    .map(|path| format!("{path}\\..\\bin;"))
                    .collect::<Vec<_>>()
                    .join(""))
                .unwrap_or("".to_owned()),
            env::var("Path").unwrap_or("".to_owned()),
        ))
        .encode_wide()
        .chain(Some(0))
        .collect();
        SetEnvironmentVariableW(PCWSTR(path.as_ptr()), PCWSTR(value.as_ptr())).unwrap();
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
