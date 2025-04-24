use dunce::canonicalize;
use std::env;
use std::io::Write;
use std::process::Command;

fn main() {
    if let Some(toolchain) = get_toolchain() {
        println!("cargo::rustc-env=RUSTOWL_TOOLCHAIN={toolchain}");
    }

    let config_sysroot = if let Ok(runtime_dir) = env::var("RUSTOWL_RUNTIME_DIR") {
        Some(runtime_dir)
    } else {
        env::var("RUSTOWL_SYSROOT").ok()
    };
    if let Some(sysroot) = config_sysroot {
        println!("cargo::rustc-env=RUSTOWL_SYSROOT={}", sysroot);
    }

    let tarball_name = format!("runtime-{}.tar.gz", get_host_tuple().unwrap());
    println!("cargo::rustc-env=RUSTOWL_TARBALL_NAME={tarball_name}");

    let sysroot = get_sysroot().unwrap();
    set_rustc_driver_path(&sysroot);
}

use std::path::Path;
// get toolchain
fn get_toolchain() -> Option<String> {
    if let Ok(toolchain) = env::var("RUSTOWL_TOOLCHAIN") {
        return Some(toolchain);
    }
    env::var("RUSTUP_TOOLCHAIN").ok()
}
fn get_host_tuple() -> Option<String> {
    match Command::new(env::var("RUSTC").unwrap())
        .arg("--print=host-tuple")
        .output()
    {
        Ok(v) => Some(String::from_utf8(v.stdout).unwrap().trim().to_string()),
        Err(_) => None,
    }
}
// output rustc_driver path
fn get_sysroot() -> Option<String> {
    match Command::new(env::var("RUSTC").unwrap())
        .arg("--print=sysroot")
        .output()
    {
        Ok(v) => Some(String::from_utf8(v.stdout).unwrap().trim().to_string()),
        Err(_) => None,
    }
}
use std::fs::read_dir;
use std::path::PathBuf;
fn recursive_read_dir(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for entry in read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            paths.extend_from_slice(&recursive_read_dir(&path));
        } else {
            paths.push(canonicalize(&path).unwrap());
        }
    }
    paths
}
fn set_rustc_driver_path(sysroot: &str) {
    for file in recursive_read_dir(sysroot) {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "rlib" | "so" | "dylib" | "dll") {
                let rel_path = file.strip_prefix(sysroot).unwrap();
                let file_name = rel_path.file_name().unwrap().to_str().unwrap();
                if file_name.contains("rustc_driver") {
                    println!(
                        "cargo::rustc-env=RUSTC_DRIVER_DIR={}",
                        rel_path.parent().unwrap().display()
                    );
                }
            }
        }
    }
}
