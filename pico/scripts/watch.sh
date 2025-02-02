#!/bin/bash

if [ "$#" -eq 2 ] && [ "$2" == "looping" ]; then
    set -x

    cargo clippy --tests --lib --target=aarch64-apple-darwin -- -D warnings
    cargo clippy

    cargo fmt
    cargo test   --lib --target=aarch64-apple-darwin --quiet

    cargo build
    cargo run

    cargo doc --document-private-items

    cargo size --bin pico-oled -- -A
else
    cargo watch -c -s "sh $(realpath $BASH_SOURCE) $1 looping"
fi
