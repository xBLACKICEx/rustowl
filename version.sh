#!/bin/bash -x

VERSION="0.1.3"

SED=$(which gsed || echo "sed")
SED_SUBSTITUTE="[0-9]\+\.[0-9]\+\.[0-9]\+/$VERSION"

# root
$SED -i -e "55 s/$SED_SUBSTITUTE/g; 62s/$SED_SUBSTITUTE/g" README.md

# RustOwl
$SED -i -e "3 s/$SED_SUBSTITUTE/g" rustowl/install.sh
$SED -i -e "3 s/$SED_SUBSTITUTE/g" rustowl/Cargo.toml

# VSCode extension
$SED -i -e "5 s/$SED_SUBSTITUTE/g" vscode/package.json
