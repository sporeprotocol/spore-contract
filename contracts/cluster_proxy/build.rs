use ckb_hash::blake2b_256;
use std::{env, fs};

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let cluster_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode)
        .join("cluster");
    let cluster = std::fs::read(cluster_path).expect("load cluster");
    let code_hash = blake2b_256(cluster);
    let file = format!("pub const CLUSTER_CODE_HASHES: [[u8; 32]; 1] = [{code_hash:?}];\n");
    fs::write("./src/hash.rs", file).unwrap();
}
