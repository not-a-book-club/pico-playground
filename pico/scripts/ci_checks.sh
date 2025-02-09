#!/bin/bash

set -ex

cargo fmt
cargo fmt --check

cargo clippy -- -D warnings

cargo clippy --tests --lib --target=aarch64-apple-darwin -- -D warnings
cargo test   --lib --target=aarch64-apple-darwin --quiet

cargo build
cargo build --release

# cargo size --bin pico-oled -- -A
# cargo size --bin pico-oled --release -- -A
