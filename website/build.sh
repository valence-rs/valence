#!/usr/bin/env sh

# This script builds the website and outputs ready-to-serve contents in ./public/
#
# Requirements:
# - zola       https://www.getzola.org/documentation/getting-started/installation/
# - cargo      https://doc.rust-lang.org/cargo/getting-started/installation.html
# - mdBook     https://rust-lang.github.io/mdBook/guide/installation.html

set -eu

# cd into this directory
cd "$(dirname "$0")"

# clean the ./public directory
rm -rf public/*

# build zola
zola build

# build mdBook
mdbook build

# build rustdoc
mkdir public/rustdoc
cargo doc --no-deps --workspace --all-features
cp -r ../target/doc/* public/rustdoc

# copy logo assets
cp ../assets/logo.svg public/valence.svg
