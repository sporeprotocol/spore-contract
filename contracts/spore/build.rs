use ckb_hash::blake2b_256;
use std::{env, fs};

fn load_code_hash(binary_name: &str, compile_mode: &str) -> [u8; 32] {
    let binary_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode)
        .join(binary_name);
    let binary = std::fs::read(binary_path).expect("load cluster");
    blake2b_256(binary)
}

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
    let cluster_code_hash = load_code_hash("cluster", &compile_mode);
    let cluster_agent_code_hash = load_code_hash("cluster_agent", &compile_mode);

    let mut cluster_code_hashes = vec![cluster_code_hash];
    // this is version v1 of cluster contract in testnet
    cluster_code_hashes.push(
        hex::decode("598d793defef36e2eeba54a9b45130e4ca92822e1d193671f490950c3b856080")
            .unwrap()
            .try_into()
            .unwrap(),
    );

    let mut content = concat_code_hashes("CLUSTER_CODE_HASHES", &cluster_code_hashes);
    content += concat_code_hashes("CLUSTER_AGENT_CODE_HASHES", &[cluster_agent_code_hash]).as_str();
    fs::write("./src/hash.rs", content).unwrap();
}
