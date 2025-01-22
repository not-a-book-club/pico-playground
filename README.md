## Pico Playground

pico pico

## Layout

This repo has three Rust packages (aka crates):

- `pico`
    - This is the binary that runs on the pico
- `runner`
    - This is a helper binary that runs on most desktop systems
- `simulations`
    - This is a utility crate which houses a Game of Life and other basic cellular automata simulations
    - https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life
    - https://en.wikipedia.org/wiki/Rule_30

## Setup

```sh
# Update to the latest stable Rust:
rustup update
rustup default stable

# Install the target triple for our Cortex M0 Raspberry Pi Pico
rustup target add thumbv6m-none-eabi

# Install tools we'll use to deploy to the pico
cargo install efl2uf2-rs

# cargo watch can be helpful to write better code
cargo install cargo-watch
```

## Working on the code

When I'm working on code, I run this in a terminal on the side:
```sh
cargo watch -c -x clippy -x fmt -x "build" -x "size --bin pico --release -- -A" -x "run" -x "doc"
```

- `cargo clippy` for lints and warnings
- `cargo fmt` to keep the code formatted consistently
- `cargo build` builds the code
- `cargo size --bin pico --release -- -A` prints size info about each section of our binary. Since we're on a micro controller, we want to keep these sizes smaller.
- `run` will build the code and use `elf2uf2-rs` to flash it to a pico in BOOTSEL. The Pi will then reboot and start running code.
- `doc` will build documentation for this crate and its dependencies, which can be helpful too.
```
