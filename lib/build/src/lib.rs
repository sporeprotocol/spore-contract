use faster_hex::hex_decode;
use std::env;
use std::fs;

#[derive(serde::Deserialize)]
pub struct FrozenVersions {
    code_hash_list: Vec<PublishedCodeHash>,
}

#[derive(serde::Deserialize)]
pub struct PublishedCodeHash {
    #[serde(default)]
    spore: String,
    #[serde(default)]
    cluster: String,
    #[serde(default)]
    cluster_proxy: String,
    #[serde(default)]
    cluster_agent: String,
    #[serde(default)]
    mutant: String,
}

fn hex_to_byte32(hex: &str) -> [u8; 32] {
    assert!(hex.len() == 64, "only accept [u8; 32] as hex string");
    let mut byte32 = [0u8; 32];
    hex_decode(hex.as_bytes(), &mut byte32).expect("hex to byte32");
    byte32
}

impl FrozenVersions {
    pub fn spore_code_hashes(&self) -> Vec<[u8; 32]> {
        self.code_hash_list
            .iter()
            .filter_map(|v| {
                if !v.spore.is_empty() {
                    Some(hex_to_byte32(&v.spore))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn cluster_code_hashes(&self) -> Vec<[u8; 32]> {
        self.code_hash_list
            .iter()
            .filter_map(|v| {
                if !v.cluster.is_empty() {
                    Some(hex_to_byte32(&v.cluster))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn cluster_proxy_code_hashes(&self) -> Vec<[u8; 32]> {
        self.code_hash_list
            .iter()
            .filter_map(|v| {
                if !v.cluster_proxy.is_empty() {
                    Some(hex_to_byte32(&v.cluster_proxy))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn cluster_agent_code_hashes(&self) -> Vec<[u8; 32]> {
        self.code_hash_list
            .iter()
            .filter_map(|v| {
                if !v.cluster_agent.is_empty() {
                    Some(hex_to_byte32(&v.cluster_agent))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn mutant_code_hashes(&self) -> Vec<[u8; 32]> {
        self.code_hash_list
            .iter()
            .filter_map(|v| {
                if !v.mutant.is_empty() {
                    Some(hex_to_byte32(&v.mutant))
                } else {
                    None
                }
            })
            .collect()
    }
}

pub fn load_frozen_toml() -> FrozenVersions {
    let net_type = if env::var("CARGO_FEATURE_RELEASE_EXPORT").is_ok() {
        "mainnet"
    } else {
        "testnet"
    };
    let frozen_path = env::current_dir()
        .unwrap()
        .join("../../deployment/frozen")
        .join(format!("{net_type}.toml"));
    let frozen = fs::read_to_string(frozen_path.clone()).expect(&format!("{} failed to load", frozen_path.to_str().unwrap()));
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
