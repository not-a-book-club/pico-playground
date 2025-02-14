#!/bin/bash
set -ex

BIN=${BIN:-"pico-oled"}

if [ "$#" -eq 2 ] && [ "$2" == "looping" ]; then
    set -x
    cargo clippy

    set -xe
    cargo clippy --tests --lib --target=aarch64-apple-darwin
    cargo fmt
    cargo test   --lib --target=aarch64-apple-darwin --quiet

    cargo build
    # cargo run
    cargo run --release --bin $BIN

    cargo doc --document-private-items

    cargo size --bin $BIN -- -A
else
    cargo watch -c -s "sh $(realpath $BASH_SOURCE) $1 looping"
fi
