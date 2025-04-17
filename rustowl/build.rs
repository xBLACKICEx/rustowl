use std::env;
use std::io::Read;
use std::process::{Command, Stdio, exit};

fn main() {
    if env::var("CARGO_FEATURE_INSTALLER").is_ok() && env::var("CARGO_FEATURE_COMPILER").is_err() {
        install_compiler();
    }

    if let Some(toolchain) = get_toolchain(".") {
        println!("cargo::rustc-env=RUSTOWL_TOOLCHAIN={toolchain}");
    }

    if let Ok(toolchain_dir) = env::var("RUSTOWL_TOOLCHAIN_DIR") {
        println!("cargo::rustc-env=RUSTOWL_TOOLCHAIN_DIR={toolchain_dir}");
    }
}

use std::path::{Path, PathBuf};
// get toolchain
fn get_toolchain(current: impl AsRef<Path>) -> Option<String> {
    if let Ok(toolchain) = env::var("RUSTOWL_TOOLCHAIN") {
        return Some(toolchain);
    }

    let child = match Command::new("rustup")
        .args(["show", "active-toolchain"])
        .env_remove("RUSTUP_TOOLCHAIN")
        .current_dir(current)
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(v) => v,
        Err(_) => return None,
    };
    let mut stdout = child.stdout.unwrap();
    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();
    let toolchain = output.split_whitespace().next().unwrap().trim().to_owned();
    Some(toolchain)
}

// compiler installer
use std::fs;
fn install_compiler() {
    let ws_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let tmp_dir = ws_root.join(".tmp");
    // copy current dir to avoid cargo's dir locking
    dir_copy(&ws_root, &tmp_dir);
    let tmp_dir = fs::canonicalize(&tmp_dir).unwrap();

    let mut rustup = Command::new("rustup")
        .args(["install"])
        .current_dir(&tmp_dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let code = rustup.wait().unwrap().code().unwrap();
    if code != 0 {
        println!("cargo::error=Toolchain for RustOwl installation failed");
        fs::remove_dir_all(&tmp_dir).unwrap();
        exit(code);
    }
    println!("cargo::warning=Toolchain for RustOwl installed");

    let toolchain = get_toolchain(&tmp_dir).unwrap();
    let mut cmd = Command::new("rustup");
    cmd.args([
        "run",
        &toolchain,
        "cargo",
        "install",
        "--path",
        ".",
        "--bin",
        "rustowlc",
        "--features",
        "compiler",
    ])
    .current_dir(&tmp_dir)
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    for (key, _) in env::vars() {
        if key.starts_with("RUSTC") || key.starts_with("CARGO") || key.starts_with("RUSTUP") {
            cmd.env_remove(key);
        }
    }
    let installer = cmd.spawn();
    let code = installer.unwrap().wait().unwrap().code().unwrap();

    // check exit code to prevent instruction reordering
    if code == 0 {
        println!("cargo::warning=RustOwl compiler installed");
        fs::remove_dir_all(&tmp_dir).ok();
    }
    if code != 0 {
        println!("cargo::error=RustOwl compiler installation failed");
        fs::remove_dir_all(&tmp_dir).ok();
        exit(code);
    }
}

fn dir_copy(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    let from = fs::canonicalize(&from).unwrap();
    let entries: Vec<_> = fs::read_dir(&from).unwrap().collect();
    fs::create_dir_all(&to).unwrap();
    let to = fs::canonicalize(&to).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let to = to.join(entry.file_name());
        let filename = to.file_name().unwrap().to_string_lossy();
        if filename == "target" || filename.starts_with(".") {
            continue;
        }
        if entry.metadata().unwrap().is_file() {
            fs::copy(entry.path(), &to).unwrap();
        } else {
            dir_copy(entry.path(), &to);
        }
    }
}
