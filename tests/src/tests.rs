use std::any::Any;
use std::io::Read;
use std::num::ParseIntError;
use ckb_testtool::builtin::ALWAYS_SUCCESS;
use super::*;
use ckb_testtool::context::Context;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::TransactionBuilder,
    core::TransactionView,
    packed::*,
    prelude::*,
};
use ckb_testtool::ckb_error::Error;
use ckb_testtool::ckb_hash::Blake2bBuilder;
use ckb_testtool::ckb_jsonrpc_types::Capacity;
use ckb_testtool::ckb_types::core::ScriptHashType;
use spore_types::{NativeNFTData};
use spore_types::generated::spore_types::{SporeData};

const MAX_CYCLES: u64 = 10_000_000;

// error numbers
const ERROR_EMPTY_ARGS: i8 = 5;

fn assert_script_error(err: Error, err_code: i8) {
    let error_string = err.to_string();
    assert!(
        error_string.contains(format!("error code {} ", err_code).as_str()),
        "error_string: {}, expected_error_code: {}",
        error_string,
        err_code
    );
}

fn build_simple_create_context(nft_content: String, nft_type: String) -> (Context, TransactionView) {
    build_create_context(nft_content.into_bytes(), nft_type)
}



fn build_create_context(nft_content: Vec<u8>, nft_type: String) -> (Context, TransactionView) {
    let nft_data: NativeNFTData = NativeNFTData {
        content: nft_content.clone(),
        content_type: nft_type.clone(),
        cluster: None
    };

    println!("NFT DATA:{:?}", nft_data);

    let serialized = SporeData::from(nft_data);
    let mut context = Context::default();
    let nft_bin: Bytes = Loader::default().load_binary("cellular");
    let nft_out_point = context.deploy_cell(nft_bin);

    let input_ckb = { serialized.total_size() } as u64;

    let output_ckb = input_ckb;

    let always_success_out_point = context.deploy_contract(ALWAYS_SUCCESS.clone());

    // build lock script
    let lock_script = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");
    let lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .lock(lock_script.clone())
            .capacity(input_ckb.pack())
            .build(), Bytes::new()
    );
    let input = CellInput::new_builder().previous_output(input_out_point).build();




    let nft_script_args: Bytes = {
        let mut blake2b = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        blake2b.update(input.as_slice());
        blake2b.update(&(0 as u64).to_le_bytes());
        let mut verify_id = [0; 32];
        blake2b.finalize(&mut verify_id);
        verify_id.to_vec().into()
    };

    let nft_script = context
        .build_script_with_hash_type(&nft_out_point, ScriptHashType::Data1, nft_script_args)
        .expect("script");

    let nft_script_dep = CellDep::new_builder().out_point(nft_out_point).build();


    let output = CellOutput::new_builder()
        .capacity(output_ckb.pack())
        .lock(lock_script.clone())
        .type_(Some(nft_script.clone()).pack())
        .build();


    let tx = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(serialized.as_slice().pack())
        .cell_dep(nft_script_dep)
        .build();

    println!("data: {:?}", format!("{:x}", serialized.as_slice().pack()));

    (context, tx)
}

#[test]
fn test_simple() {
    let (mut context, tx) =
        build_simple_create_context(
            "THIS IS A TEST NFT".to_string(),
            "plain/text".to_string()
        );

    let tx = context.complete_tx(tx);

    // run
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");

}

#[test]
fn test_empty_content() {
    let (mut context, tx) =
        build_simple_create_context(
            "".to_string(),
            "plain/text".to_string()
        );
    let tx = context.complete_tx(tx);

    // run
    assert!(context
        .verify_tx(&tx, MAX_CYCLES).is_err()
    );
}

#[test]
fn test_read_file() {
    use std::io;
    use std::io::Read;
    use std::io::BufReader;
    use std::fs::File;
    let f = File::open("res/test.jpg");
    if f.is_ok() {
        let f = f.unwrap();
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).expect("Error read to end");

        build_create_context(buffer, "image/jpeg".to_string());
    } else {
        println!("Error while reading file!");
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

#[test]
fn test_read_base64() {

    let (mut context, tx) = build_simple_create_context("/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAoHBwgHBgoICAgLCgoLDhgQDg0NDh0VFhEYIx8lJCIfIiEmKzcvJik0KSEiMEExNDk7Pj4+JS5ESUM8SDc9Pjv/2wBDAQoLCw4NDhwQEBw7KCIoOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozv/wAARCAAKAAoDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAABAf/xAAfEAADAAICAwEBAAAAAAAAAAABAgMEBQARBiExQRT/xAAUAQEAAAAAAAAAAAAAAAAAAAAA/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAwDAQACEQMRAD8AZHc7XC1vm2wts8x8ds7Y4Ume7H+N0QtAoSepqSzp69ljEAfeUPxO1cnw7S3vV61rr4PSjsWZ2M1JJJ+kn94p9NqqYuRivrMNsfKqbZEjBSlnJBLMOumbsA9n36HExjLGhOEJJKUlCTmihVRQOgAB8AH5wP/Z".to_string(), "image/jpeg".to_string());
    let view = context.complete_tx(tx);
    context.verify_tx(&view, MAX_CYCLES).expect("Error tx");
}

#[test]
fn test_decode_hex() {
    let hex_str = "ba010000100000001e000000ba0100000a000000696d6167652f6a70656798010000ffd8ffe000104a46494600010101004800480000ffdb0043000a07070807060a0808080b0a0a0b0e18100e0d0d0e1d15161118231f2524221f2221262b372f26293429212230413134393b3e3e3e252e4449433c48373d3e3bffdb0043010a0b0b0e0d0e1c10101c3b2822283b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3bffc0001108000a000a03012200021101031101ffc4001500010100000000000000000000000000000407ffc4001f1000030002020301010000000000000000010203040500110621314114ffc40014010100000000000000000000000000000000ffc40014110100000000000000000000000000000000ffda000c03010002110311003f0064773b5c2d6f9b6c2db3cc7c76ced8e1499eec7f8dd10b40a127a9a92ce9ebd9631007de50fc4ed5c9f0ed2def57ad6baf83d28ec599d8cd49249fa49fde29f4daaa62e462beb30db1f2aa6d91230529672412cc3ae99bb00f67dfa1c4c632c684e10924a5250939a28554503a0001f001f9c0ffd9";
    let data = decode_hex(hex_str).unwrap();
    let nft = SporeData::from_slice(data.as_slice()).expect("error parse");

    println!("content-type: {:?}, content: {:?}, cluster: {:?}", nft.content_type(), nft.content(), nft.cluster());
}

#[test]
fn test_error_type() {
    let error_nft_types = [
        "plain/;",
        "text",
        ";",
        "-",
        "plain/",
        "plain/test;;test=;",
    ];

    for content_type in error_nft_types {

        let (mut context, tx) =
            build_simple_create_context(
                "THIS IS A TEST NFT".to_string(),
                content_type.to_string()
            );
        let tx = context.complete_tx(tx);

        assert!(context
            .verify_tx(&tx, MAX_CYCLES).is_err()
        );
    }

}