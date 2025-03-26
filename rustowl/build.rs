use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    let mut stdout = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .stdout
        .unwrap();
    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();
    let toolchain = output.split("\n").nth(0).unwrap().trim();
    let path = PathBuf::from(toolchain).join(env!("RUSTUP_HOME"));
    println!("cargo::rustc-env=RUSTOWL_TOOLCHAIN_PATH={}", path.display());
}
