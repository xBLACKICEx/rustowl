#!/bin/bash

VERSION="v0.0.3pre"

function install_rust() {
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -q -y --profile minimal
}

function install_rustowl() {
    if ! which curl > /dev/null 2>&1; then
        echo "curl command not found; error"
        return
    fi
    if ! which unzip > /dev/null 2>&1; then
        echo "unzip command not found; error"
        return
    fi
    cd "$(mktemp -d)"
    curl -L "https://github.com/cordx56/rustowl/releases/download/$VERSION/rustowl.zip" > rustowl.zip
    unzip rustowl.zip
    cd ./rustowl
    cargo install --path . --locked
}

install_rustowl
