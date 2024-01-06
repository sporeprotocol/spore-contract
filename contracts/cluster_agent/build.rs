use ckb_hash::blake2b_256;
use std::{env, fs};

use spore_build_tools::{concat_code_hashes, load_frozen_toml};

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let cluster_proxy_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode)
        .join("cluster_proxy");
    let cluster_proxy = std::fs::read(cluster_proxy_path).expect("load cluster_proxy");
    let code_hash = blake2b_256(cluster_proxy);

    let frozen = load_frozen_toml();
    let code_hashes = [frozen.cluster_proxy_code_hashes(), vec![code_hash]].concat();

    let file = concat_code_hashes("CLUSTER_PROXY_CODE_HASHES", &code_hashes);
    fs::write("./src/hash.rs", file).unwrap();
}
