use ckb_hash::blake2b_256;
use std::{env, fs};

pub fn main() {
    let compile_mode = env::var("PROFILE").unwrap();
    let libckblua_path = env::current_dir().unwrap().join("lua/libckblua.so");
    let libckblua = std::fs::read(libckblua_path).expect("load libckblua.so");
    let code_hash = blake2b_256(&libckblua);
    let file = format!("pub const CKB_LUA_LIB_CODE_HASH: [u8; 32] = {code_hash:?};\n");
    fs::write("./src/hash.rs", file).unwrap();

    let build_path = env::current_dir()
        .unwrap()
        .join("../../build")
        .join(compile_mode);
    fs::create_dir_all(&build_path).unwrap();
    fs::write(build_path.join("libckblua.so"), libckblua)
        .expect("copy libckblua.so for capsule test");
}
