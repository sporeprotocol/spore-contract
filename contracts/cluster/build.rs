use ckb_hash::blake2b_256;
use std::{env, fs};

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let spore_extension_lua_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode)
        .join("spore_extension_lua");
    let spore_extension_lua =
        std::fs::read(spore_extension_lua_path).expect("load spore_extension_lua");
    let code_hash = blake2b_256(spore_extension_lua);
    let file = format!("pub const SPORE_EXTENSION_LUA: [[u8; 32]; 1] = [{code_hash:?}];\n");
    fs::write("./src/hash.rs", file).unwrap();
}
