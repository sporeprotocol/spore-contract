use ckb_hash::blake2b_256;
use std::{env, fs};

use spore_build_tools::{concat_code_hashes, load_frozen_toml};

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let cluster_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(&compile_mode)
        .join("cluster");
    let cluster = std::fs::read(cluster_path).expect("load cluster");
    let code_hash = blake2b_256(cluster);

    let frozen = load_frozen_toml();
    let cluster_code_hashes = [frozen.cluster_code_hashes(), vec![code_hash]].concat();

    let content = concat_code_hashes("CLUSTER_CODE_HASHES", &cluster_code_hashes);
    fs::write("./src/hash.rs", content).unwrap();
}
