#!/bin/bash

bin=pico-oled

set -ex
cargo build --release --bin $bin
cargo size  --release --bin $bin -- -A

cargo_bloat="cargo bloat --release --bin $bin --split-std"
# Aggregate by crate
$cargo_bloat --crates

# Just our stuff
$cargo_bloat -n 30 --filter pico
$cargo_bloat -n 30 --filter simulations

# Everything
$cargo_bloat -n 30
