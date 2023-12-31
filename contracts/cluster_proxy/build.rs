use ckb_hash::blake2b_256;
use std::{env, fs};

pub fn concat_code_hashes(var_name: &str, code_hashes: &[[u8; 32]]) -> String {
    let mut content = format!(
        "pub const {var_name}: [[u8; 32]; {}] = [",
        code_hashes.len()
    );
    code_hashes.into_iter().for_each(|v| {
        content += &format!("{v:?},");
    });
    content += "];\n";
    content
}

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let cluster_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(&compile_mode)
        .join("cluster");
    let cluster = std::fs::read(cluster_path).expect("load cluster");
    let code_hash = blake2b_256(cluster);

    let mut cluster_code_hashes = vec![code_hash];
    // this is version v1 of cluster contract in testnet
    cluster_code_hashes.push(
        hex::decode("598d793defef36e2eeba54a9b45130e4ca92822e1d193671f490950c3b856080")
            .unwrap()
            .try_into()
            .unwrap(),
    );

    let content = concat_code_hashes("CLUSTER_CODE_HASHES", &cluster_code_hashes);
    fs::write("./src/hash.rs", content).unwrap();
}
