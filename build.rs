use clap_complete::generate_to;
use std::env;
use std::fs;
use std::io::Error;
use std::process::Command;

include!("src/cli.rs");
include!("src/shells.rs");

fn main() -> Result<(), Error> {
    if env::var("RUSTOWL_TOOLCHAIN").is_err() {
        println!("cargo::rustc-env=RUSTOWL_TOOLCHAIN={}", get_toolchain());
    }

    if let Ok(sysroot) = env::var("RUSTOWL_RUNTIME_DIRS") {
        println!("cargo::rustc-env=RUSTOWL_RUNTIME_DIRS={}", sysroot);
    }

    #[cfg(not(windows))]
    let tarball_name = format!("rustowl-{}.tar.gz", get_host_tuple().unwrap());

    #[cfg(windows)]
    let tarball_name = format!("rustowl-{}.zip", get_host_tuple().unwrap());

    println!("cargo::rustc-env=RUSTOWL_ARCHIVE_NAME={tarball_name}");

    let sysroot = env::var("RUSTOWL_SYSROOT").unwrap_or_else(|_| get_sysroot().unwrap());
    set_rustc_driver_path(&sysroot);

    let out_dir = Path::new(&env::var("OUT_DIR").expect("OUT_DIR unset. Expected path."))
        .join("rustowl-build-time-out");
    let mut cmd = cli();
    let completion_out_dir = out_dir.join("completions");
    fs::create_dir_all(&completion_out_dir)?;

    for shell in Shell::value_variants() {
        generate_to(*shell, &mut cmd, "rustowl", &completion_out_dir)?;
    }
    let man_out_dir = out_dir.join("man");
    fs::create_dir_all(&man_out_dir)?;
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(man_out_dir.join("rustowl.1"), buffer)?;

    Ok(())
}

// get toolchain
fn get_toolchain() -> String {
    env::var("RUSTUP_TOOLCHAIN").expect("RUSTUP_TOOLCHAIN unset. Expected version.")
}
fn get_host_tuple() -> Option<String> {
    match Command::new(env::var("RUSTC").expect("RUSTC unset. Expected rustc path."))
        .arg("--print=host-tuple")
        .output()
    {
        Ok(v) => Some(String::from_utf8(v.stdout).unwrap().trim().to_string()),
        Err(_) => None,
    }
}
// output rustc_driver path
fn get_sysroot() -> Option<String> {
    match Command::new(env::var("RUSTC").expect("RUSTC unset. Expected rustc path."))
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
            paths.push(path);
        }
    }
    paths
}
fn set_rustc_driver_path(sysroot: &str) {
    for file in recursive_read_dir(sysroot) {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "rlib" | "so" | "dylib" | "dll") {
                let rel_path = file.strip_prefix(sysroot).unwrap();
                let file_name = rel_path.file_name().unwrap().to_string_lossy();
                if file_name.contains("rustc_driver-") {
                    println!("cargo::rustc-env=RUSTC_DRIVER_NAME={file_name}");
                }
            }
        }
    }
}
