#!/bin/bash

export RUSTUP_HOME="$(pwd)/rustup"
export CARGO_HOME="$(pwd)/cargo"
export PATH="$CARGO_HOME/bin:$PATH"

function install_rust() {
    mkdir -p rustup
    mkdir -p cargo

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | env RUSTUP_HOME="$RUSTUP_HOME" CARGO_HOME="$CARGO_HOME" sh -s -- -q -y --default-toolchain nightly-2024-10-31 --profile minimal -c rustc-dev
}

function install_rustowl() {
    rm -rf "./rustowl-*"
    curl -L "https://github.com/cordx56/rustowl/releases/latest/download/rustowl-server.zip" > rustowl-server.zip
    unzip rustowl.zip
}

export RUSTOWL_SERVER_PATH="$(pwd)/rustowl-server"

function run() {
    cd "$RUSTOWL_SERVER_PATH" && cargo run --release
}

(which rustup || install_rust) && \
    ([ -d "$RUSTOWL_SERVER_PATH" ] || install_rustowl) && \

if [ "$1" = "run" ]; then
    run
fi
