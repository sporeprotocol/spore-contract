use spore_build_tools::{concat_code_hashes, deployment::Deployment};
use std::{env, fs};

fn main() {
    let deployment = Deployment::load();

    let mut content = String::new();
    content += &concat_code_hashes(
        "SPORE_EXTENSION_RUST_TYPE_HASHES",
        &[deployment.type_hash("mutant")],
    );
    fs::write(
        std::path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("hash.rs"),
        content,
    )
    .unwrap();
}
