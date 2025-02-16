#!/bin/bash

cargo run --release --                          \
    frames/                                     \
    --frame-rate-div=3                          \
    --height 64                                 \
    -o ~/code/me/pico-life/pico/
