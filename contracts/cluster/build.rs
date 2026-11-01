use ckb_hash::blake2b_256;
use std::{env, fs};

use spore_build_tools::concat_code_hashes;

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let rust_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(&compile_mode)
        .join("spore_extension_rust");
    println!("cargo:rerun-if-changed={}", rust_path.display());
    let rust_code_hash = blake2b_256(std::fs::read(rust_path).expect("load spore_extension_rust"));

    let file = concat_code_hashes("SPORE_EXTENSION_RUST", &[rust_code_hash]);
    fs::write(
        std::path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("hash.rs"),
        file,
    )
    .unwrap();
}
