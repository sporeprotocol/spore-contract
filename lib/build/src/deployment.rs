//! Stable code-Cell identities supplied by an explicit deployment plan.
use serde::Deserialize;
use std::{collections::BTreeMap, env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    pub format: u32,
    pub profile: String,
    pub type_ids: BTreeMap<String, String>,
}

pub const ROLES: [&str; 5] = [
    "spore",
    "cluster",
    "cluster_proxy",
    "cluster_agent",
    "mutant",
];

impl Deployment {
    pub fn parse(text: &str) -> Self {
        let deployment: Self = toml::from_str(text).expect("invalid Type ID deployment plan");
        assert_eq!(
            deployment.format, 1,
            "unsupported Type ID deployment format"
        );
        assert!(
            !deployment.profile.is_empty(),
            "deployment profile is required"
        );
        assert_eq!(
            deployment.type_ids.len(),
            ROLES.len(),
            "all five code roles are required"
        );
        let mut seen = std::collections::BTreeSet::new();
        for role in ROLES {
            let id = deployment.args(role);
            assert!(seen.insert(id), "code roles must have distinct Type IDs");
        }
        deployment
    }

    pub fn load() -> Self {
        println!("cargo:rerun-if-env-changed=SPORE_DEPLOYMENT_MANIFEST");
        let configured = env::var_os("SPORE_DEPLOYMENT_MANIFEST").expect(
            "set SPORE_DEPLOYMENT_MANIFEST to a reviewed Type ID plan; VM tests explicitly use deployment/type-ids.fixture.toml",
        );
        let configured = PathBuf::from(configured);
        let path = if configured.is_absolute() {
            configured
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(configured)
        };
        println!("cargo:rerun-if-changed={}", path.display());
        Self::parse(&fs::read_to_string(path).expect("read Type ID deployment plan"))
    }

    pub fn args(&self, role: &str) -> [u8; 32] {
        let value = self.type_ids.get(role).expect("missing code Type ID");
        let value = value.strip_prefix("0x").unwrap_or(value);
        assert_eq!(value.len(), 64, "code Type ID must be 32 bytes");
        let mut bytes = [0u8; 32];
        faster_hex::hex_decode(value.as_bytes(), &mut bytes).expect("invalid code Type ID hex");
        bytes
    }

    pub fn type_hash(&self, role: &str) -> [u8; 32] {
        type_id_script_hash(self.args(role))
    }
}

/// Hash the standard Type ID Script, using its exact Molecule encoding.
/// This hashes the code Cell's type script, not its ELF data or governance Lock.
pub fn type_id_script_hash(args: [u8; 32]) -> [u8; 32] {
    let mut script = Vec::with_capacity(85);
    for n in [85u32, 16, 48, 49] {
        script.extend_from_slice(&n.to_le_bytes());
    }
    let mut code_hash = [0u8; 32];
    code_hash[25..].copy_from_slice(b"TYPE_ID");
    script.extend_from_slice(&code_hash);
    script.push(1); // standard Type ID itself uses hash_type: type
    script.extend_from_slice(&32u32.to_le_bytes());
    script.extend_from_slice(&args);
    ckb_hash::blake2b_256(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../deployment/type-ids.fixture.toml");

    #[test]
    fn standard_type_id_hash_matches_the_ccc_encoding() {
        let deployment = Deployment::parse(FIXTURE);
        let mut expected = [0u8; 32];
        faster_hex::hex_decode(
            b"946f717cf6324bcd09ebfbdfabab3cd41d49d6b2091f4c529d58ea3aa8356b5b",
            &mut expected,
        )
        .unwrap();
        assert_eq!(deployment.type_hash("spore"), expected);
    }

    #[test]
    #[should_panic(expected = "distinct Type IDs")]
    fn deployment_roles_cannot_alias() {
        let first = "01".repeat(32);
        let duplicate = FIXTURE.replace(&"02".repeat(32), &first);
        Deployment::parse(&duplicate);
    }

    #[test]
    #[should_panic(expected = "unsupported Type ID deployment format")]
    fn unknown_formats_are_rejected() {
        Deployment::parse(&FIXTURE.replace("format = 1", "format = 2"));
    }
}
