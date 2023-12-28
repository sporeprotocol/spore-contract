use ckb_hash::blake2b_256;
use std::{env, fs};

fn load_code_hash(binary_name: &str) -> [u8; 32] {
    let compile_mode = env::var("PROFILE").unwrap();
    let binary_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode)
        .join(binary_name);
    let binary = std::fs::read(binary_path).expect("load cluster");
    blake2b_256(binary)
}

pub fn main() {
    let cluster_code_hash = load_code_hash("cluster");
    let cluster_agent_code_hash = load_code_hash("cluster_agent");

    let file = format!("pub const CLUSTER_CODE_HASHES: [[u8; 32]; 1] = [{cluster_code_hash:?}];\npub const CLUSTER_AGENT_CODE_HASHES: [[u8; 32]; 1] = [{cluster_agent_code_hash:?}];\n");
    fs::write("./src/hash.rs", file).unwrap();
}
