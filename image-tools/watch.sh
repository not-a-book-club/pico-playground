#!/bin/bash

if [ "$#" -eq 2 ] && [ "$2" == "looping" ]; then
    set -xe

    cargo clippy --all-features
    cargo clippy --no-default-features
    cargo clippy --no-default-features --features="encoder"
    cargo clippy --no-default-features --features="decoder"
    cargo clippy --no-default-features --features="std"
    cargo clippy --no-default-features --features="decoder" --target="thumbv6m-none-eabi"

    cargo fmt
    cargo nextest run

    cargo build
    cargo run -- --help
    ./run.sh

    pushd ../pico
    cargo clippy
    cargo fmt
    cargo build --release --bin bad-apple
    cargo run   --release --bin bad-apple || true

    popd
else
    cargo watch -c -s "sh $(realpath $BASH_SOURCE) $1 looping"
fi
