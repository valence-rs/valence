#!/usr/bin/env sh

# This script builds the website and outputs ready-to-serve contents in ./public/
# This script must be run from this directory.
#
# Requirements:
# - zola       https://www.getzola.org/documentation/getting-started/installation/
# - cargo      https://doc.rust-lang.org/cargo/getting-started/installation.html
# - mdBook     https://rust-lang.github.io/mdBook/guide/installation.html

# clean the ./public directory
rm -rf public/*

# build zola
zola build

# build mdBook
mdbook build

# build rustdoc
cargo doc --no-deps --workspace --all-features
cp -r ../target/doc/* public/rustdoc
