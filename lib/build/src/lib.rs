use faster_hex::hex_decode;
use std::env;
use std::fs;

#[derive(serde::Deserialize)]
struct CodeHashList {
    code_hash_list: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct PublishedCodeHash {
    spore: CodeHashList,
    cluster: CodeHashList,
    cluster_proxy: CodeHashList,
    cluster_agent: CodeHashList,
    mutant: CodeHashList,
}

fn hex_to_byte32(hex: &str) -> [u8; 32] {
    assert!(hex.len() == 64, "only accept [u8; 32] as hex string");
    let mut byte32 = [0u8; 32];
    hex_decode(hex.as_bytes(), &mut byte32).expect("hex to byte32");
    byte32
}

impl PublishedCodeHash {
    pub fn spore_code_hashes(&self) -> Vec<[u8; 32]> {
        self.spore
            .code_hash_list
            .iter()
            .map(|v| hex_to_byte32(v))
            .collect()
    }

    pub fn cluster_code_hashes(&self) -> Vec<[u8; 32]> {
        self.cluster
            .code_hash_list
            .iter()
            .map(|v| hex_to_byte32(v))
            .collect()
    }

    pub fn cluster_proxy_code_hashes(&self) -> Vec<[u8; 32]> {
        self.cluster_proxy
            .code_hash_list
            .iter()
            .map(|v| hex_to_byte32(v))
            .collect()
    }

    pub fn cluster_agent_code_hashes(&self) -> Vec<[u8; 32]> {
        self.cluster_agent
            .code_hash_list
            .iter()
            .map(|v| hex_to_byte32(v))
            .collect()
    }

    pub fn mutant_code_hashes(&self) -> Vec<[u8; 32]> {
        self.mutant
            .code_hash_list
            .iter()
            .map(|v| hex_to_byte32(v))
            .collect()
    }
}

pub fn load_frozen_toml() -> PublishedCodeHash {
    let net_type = if env::var("CARGO_FEATURE_RELEASE_EXPORT").is_ok() {
        "mainnet"
    } else {
        "testnet"
    };
    let frozen_path = env::current_dir()
        .unwrap()
        .join("../../deployment/frozen")
        .join(format!("{net_type}.toml"));
    let frozen = fs::read_to_string(frozen_path).unwrap();
    toml::from_str(&frozen).unwrap()
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
