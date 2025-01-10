#![feature(rustc_private)]

use std::collections::HashMap;
use std::env;
use std::process::{Command, Stdio};

fn main() {
    simple_logger::init().unwrap();
    let args: Vec<_> = env::args().collect();
    if args.len() == 1 {
        let output = Command::new("cargo")
            .env("RUSTC_WORKSPACE_WRAPPER", &args[0])
            .arg("rustc")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        let objs = String::from_utf8_lossy(&output.stdout)
            .split("\n")
            .filter(|v| 0 < v.len())
            .map(|v| serde_json::from_str::<HashMap<String, serde_json::Value>>(v).unwrap())
            .fold(HashMap::new(), |mut acc, x| {
                acc.extend(x);
                acc
            });
        println!("{}", serde_json::to_string(&objs).unwrap());
    } else {
        rustowl_core::run_compiler(env::args().skip(1).collect::<Vec<_>>().as_slice());
    }
}
