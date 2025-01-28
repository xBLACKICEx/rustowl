use models::Workspace;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    simple_logger::init().unwrap();
    let args: Vec<_> = env::args().collect();
    let self_path = PathBuf::from(&args[0]);
    let output = Command::new("rustup")
        .env(
            "RUSTC_WORKSPACE_WRAPPER",
            self_path.with_file_name("rustowlc"),
        )
        .env_remove("RUSTC_WRAPPER")
        .arg("run")
        .arg("nightly-2024-10-31")
        .arg("cargo")
        .arg("check")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    let objs = String::from_utf8_lossy(&output.stdout)
        .split("\n")
        .filter(|v| 0 < v.len())
        .map(|v| serde_json::from_str::<Workspace>(v).unwrap())
        .reduce(|acc, x| acc.merge(x));
    println!("{}", serde_json::to_string(&objs).unwrap());
}
