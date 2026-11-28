use spore_build_tools::{concat_code_hashes, deployment::Deployment, load_frozen_toml};
use std::{env, fs};

fn main() {
    let deployment = Deployment::load();
    let frozen = load_frozen_toml();
    let mut content = String::new();
    content += &concat_code_hashes("CLUSTER_CODE_HASHES", &frozen.cluster_code_hashes());
    content += &concat_code_hashes("CLUSTER_TYPE_HASHES", &[deployment.type_hash("cluster")]);
    fs::write(
        std::path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("hash.rs"),
        content,
    )
    .unwrap();
}
