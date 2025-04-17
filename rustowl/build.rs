use std::env;
use std::io::Read;
use std::process::{Command, Stdio};

fn main() {
    if let Some(toolchain) = get_toolchain(".") {
        println!("cargo::rustc-env=RUSTOWL_TOOLCHAIN={toolchain}");
    }

    output_lib_path("rustc_driver", "dylib");
    let sysroot = get_sysroot().unwrap();
    compress_toolchain(&sysroot);
}

use std::path::Path;
// get toolchain
fn get_toolchain(current: impl AsRef<Path>) -> Option<String> {
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
use std::fs::{canonicalize, read_dir};
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
fn output_lib_path(lib: &str, ext: &str) {
    let sysroot = get_sysroot().unwrap();
    let files = recursive_read_dir(&sysroot);
    let rustc_lib = files
        .into_iter()
        .find(|p| {
            let file_name = p.file_name().unwrap().to_str().unwrap();
            file_name.starts_with(&format!("lib{lib}-")) && file_name.ends_with(&format!(".{ext}"))
        })
        .unwrap();
    println!(
        "cargo::rustc-env={}_{}_PATH={}",
        lib.to_uppercase(),
        ext.to_uppercase(),
        rustc_lib.display()
    );
}

fn compress_toolchain(sysroot: &str) {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::fs::File;
    use tar::Builder;

    let path = canonicalize(".").unwrap().join("toolchain.tar.gz");
    let tar_gz = File::create(&path).unwrap();
    let enc = GzEncoder::new(tar_gz, Compression::best());
    let mut tar_builder = Builder::new(enc);

    for file in recursive_read_dir(sysroot) {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "rlib" | "so" | "dylib" | "dll") {
                let rel_path = file.strip_prefix(sysroot).unwrap();
                let file_name = rel_path.file_name().unwrap().to_str().unwrap();
                if file_name.starts_with("librustc_driver") {
                    println!(
                        "cargo::rustc-env=RUSTC_DRIVER_DIR={}",
                        rel_path.parent().unwrap().display()
                    );
                }
                tar_builder.append_path_with_name(&file, rel_path).unwrap();
            }
        }
    }

    let enc = tar_builder.into_inner().unwrap();
    enc.finish().unwrap();

    println!("cargo::rustc-env=TOOLCHAIN_TARBALL_PATH={}", path.display());
}
