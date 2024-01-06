use ckb_hash::blake2b_256;
use std::{env, fs};

use spore_build_tools::{concat_code_hashes, load_frozen_toml};

fn load_code_hash(binary_name: &str, compile_mode: &str) -> [u8; 32] {
    let binary_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode)
        .join(binary_name);
    let binary = std::fs::read(binary_path).expect("load cluster");
    blake2b_256(binary)
}

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let cluster_code_hash = load_code_hash("cluster", &compile_mode);
    let cluster_agent_code_hash = load_code_hash("cluster_agent", &compile_mode);

    let frozen = load_frozen_toml();
    let cluster_code_hashes = [frozen.cluster_code_hashes(), vec![cluster_code_hash]].concat();
    let cluster_agent_code_hashes = [
        frozen.cluster_agent_code_hashes(),
        vec![cluster_agent_code_hash],
    ]
    .concat();

    let mut content = concat_code_hashes("CLUSTER_CODE_HASHES", &cluster_code_hashes);
    content += concat_code_hashes("CLUSTER_AGENT_CODE_HASHES", &cluster_agent_code_hashes).as_str();
    fs::write("./src/hash.rs", content).unwrap();
}
