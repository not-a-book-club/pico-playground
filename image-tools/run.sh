#!/bin/bash

cargo run --release --                          \
    compress                                    \
    frames/                                     \
    --height 64                                 \
    -o ../pico/bad-apple.video                  \
    $*
