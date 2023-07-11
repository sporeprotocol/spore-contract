use std::io::Read;
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
use cellular_types::{NativeNFTData};
use cellular_types::generated::cellular_types::NFTData;

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
    let nft_data: NativeNFTData = NativeNFTData {
        content: nft_content.clone(),
        content_type: nft_type.clone(),
        group: None
    };

    println!("NFT DATA:{:?}", nft_data);

    let serialized = NFTData::from(nft_data);

    //let serialized = NFTData::from(nft_data);
    //let serialized =  cellular_types::generated::cellular_types::NFTData::from(nft_data);

    let mut context = Context::default();
    let nft_bin: Bytes = Loader::default().load_binary("cellular");
    let nft_out_point = context.deploy_cell(nft_bin);

    let input_ckb = { nft_type.len() + nft_content.len() } as u64;

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