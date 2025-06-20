use std::env;
use std::path::PathBuf;

fn main() {
    export_build_metadata();

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if (target_arch == "arm") && (target_os == "none") {
        std::fs::write(out.join("memory.x"), include_str!("memory.x")).unwrap();

        println!("cargo:rustc-link-search={}", out.display());
        println!("cargo:rerun-if-changed=memory.x");

        // `--nmagic` is required if memory section addresses are not aligned to 0x10000,
        // for example the FLASH and RAM sections in your `memory.x`.
        // See https://github.com/rust-embedded/cortex-m-quickstart/pull/95
        println!("cargo:rustc-link-arg=--nmagic");

        // Set the linker script to the one provided by cortex-m-rt.
        println!("cargo:rustc-link-arg=-Tlink.x");

        // Set the linker script to the one provided by defmt.
        println!("cargo:rustc-link-arg=-Tdefmt.x");
    }
}

fn export_build_metadata() {
    use vergen_git2::*;

    // For a full list of things available, see:
    //      https://docs.rs/vergen-git2/latest/vergen_git2/
    Emitter::default()
        .add_instructions(&BuildBuilder::all_build().unwrap())
        .unwrap()
        .add_instructions(&CargoBuilder::all_cargo().unwrap())
        .unwrap()
        .add_instructions(&Git2Builder::all_git().unwrap())
        .unwrap()
        .emit()
        .unwrap();
}
