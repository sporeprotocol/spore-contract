use ckb_testtool::ckb_types::bytes::Bytes;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[allow(dead_code)]
const MAX_CYCLES: u64 = 10_000_000;

#[cfg(test)]
mod tests;
mod utils;

const TEST_ENV_VAR: &str = "CAPSULE_TEST_ENV";

pub enum TestEnv {
    Debug,
    Release,
}

impl FromStr for TestEnv {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(TestEnv::Debug),
            "release" => Ok(TestEnv::Release),
            _ => Err("no match"),
        }
    }
}

pub struct Loader(PathBuf);

impl Default for Loader {
    fn default() -> Self {
        let test_env = match env::var(TEST_ENV_VAR) {
            Ok(val) => val.parse().expect("test env"),
            Err(_) => TestEnv::Debug,
        };
        Self::with_test_env(test_env)
    }
}

impl Loader {
    fn with_test_env(env: TestEnv) -> Self {
        let load_prefix = match env {
            TestEnv::Debug => "debug",
            TestEnv::Release => "release",
        };
        let dir = env::current_dir().unwrap();
        let mut base_path = PathBuf::new();
        base_path.push(dir);
        base_path.push("..");
        base_path.push("build");
        base_path.push(load_prefix);
        Loader(base_path)
    }

    #[cfg(test)]
    fn is_testnet(&self) -> bool {
        let network = fs::read_to_string(self.0.join("network"))
            .expect("staged contract network; run scripts/build-contracts.sh first");
        let network = network.trim();
        // CI can assert which network it staged. The build selects the fixture.
        if let Ok(expected) = env::var("SPORE_TEST_CLUSTER_NETWORK") {
            assert_eq!(network, expected, "staged contract network mismatch");
        }
        match network {
            "testnet" => true,
            "mainnet" => false,
            other => panic!("unknown staged contract network {other}"),
        }
    }

    pub fn load_binary(&self, name: &str) -> Bytes {
        let mut path = self.0.clone();
        path.push(name);
        fs::read(path).expect("binary").into()
    }
}
