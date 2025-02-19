#!/bin/bash

cargo run --release --                          \
    frames/                                     \
    --height 64                                 \
    -o ../pico/bad-apple.video
