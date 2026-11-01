use ckb_hash::blake2b_256;
use std::{env, fs};

use spore_build_tools::{concat_code_hashes, load_frozen_toml};

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let spore_extension_lua_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(&compile_mode)
        .join("spore_extension_lua");
    let spore_extension_lua =
        std::fs::read(&spore_extension_lua_path).expect("load spore_extension_lua");
    let code_hash = blake2b_256(spore_extension_lua);
    let rust_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(&compile_mode)
        .join("spore_extension_rust");
    println!("cargo:rerun-if-changed={}", rust_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        spore_extension_lua_path.display()
    );
    let rust_code_hash = blake2b_256(std::fs::read(rust_path).expect("load spore_extension_rust"));

    let frozen = load_frozen_toml();
    let code_hashes = [frozen.mutant_code_hashes(), vec![code_hash]].concat();

    let mut file = concat_code_hashes("SPORE_EXTENSION_LUA", &code_hashes);
    file += &concat_code_hashes("SPORE_EXTENSION_RUST", &[rust_code_hash]);
    fs::write(
        std::path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("hash.rs"),
        file,
    )
    .unwrap();
}
