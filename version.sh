#!/bin/bash -x

VERSION="0.3.2-alpha.1"

SED=$(which gsed || echo "sed")
SED_SUBSTITUTE='[0-9]\+\.[0-9]\+\.[0-9]\+[0-9|a-z|A-Z|.-]*/'$VERSION

# RustOwl
$SED -i -e "3 s/$SED_SUBSTITUTE/g" Cargo.toml

# VSCode extension
$SED -i -e "5 s/$SED_SUBSTITUTE/g" vscode/package.json

# AUR PKGBUILD
$SED -i -e "5 s/$SED_SUBSTITUTE/g" aur/PKGBUILD
