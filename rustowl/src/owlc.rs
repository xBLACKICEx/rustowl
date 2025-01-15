#![feature(rustc_private)]

use std::process::exit;

fn main() {
    simple_logger::init().unwrap();
    exit(rustowl_core::run_compiler())
}
