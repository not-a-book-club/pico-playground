#!/bin/bash

cargo run --release --                          \
    frames/                                     \
    --frame-rate-div=3                          \
    --height 64                                 \
    -o ../pico/bad-apple.video
