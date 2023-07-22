use std::any::Any;
use std::io::Read;
use std::num::ParseIntError;
use ckb_testtool::builtin::ALWAYS_SUCCESS;
use super::*;
use ckb_testtool::context::Context;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, core::TransactionView, packed::*, packed, prelude::*};
use ckb_testtool::ckb_error::Error;
use ckb_testtool::ckb_hash::Blake2bBuilder;
use ckb_testtool::ckb_jsonrpc_types::Capacity;
use ckb_testtool::ckb_types::core::cell::ResolvedDep::Cell;
use ckb_testtool::ckb_types::core::ScriptHashType;
use spore_types::{NativeNFTData};
use spore_types::generated::spore_types::{ClusterData, SporeData};
use hex;

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

fn build_create_context_with_cluster(nft_content: Vec<u8>, nft_type: String, cluster_id: String) -> (Context, TransactionView) {
    let nft_data: NativeNFTData = NativeNFTData {
        content: nft_content.clone(),
        content_type: nft_type.clone(),
        cluster: Some(cluster_id.clone()),
    };

    let dummy_cluster_name = "Spore Cluster!";
    let dummy_cluster_description = "Spore Description!";

    let cluster_data = ClusterData::new_builder()
        .name(dummy_cluster_name.as_bytes().into())
        .description(dummy_cluster_description.as_bytes().into())
        .build();

    let serialized = SporeData::from(nft_data);
    let mut context = Context::default();
    let nft_bin: Bytes = Loader::default().load_binary("spore");
    let nft_out_point = context.deploy_cell(nft_bin);
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
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

    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();


    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .lock(lock_script.clone())
            .capacity(input_ckb.pack())
            .build(), Bytes::new()
    );

    let cluster_script = context
        .build_script_with_hash_type(
            &cluster_out_point,
            ScriptHashType::Data1,
            cluster_id.into()
        )
        .expect("cluster script");

    let cluster_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((cluster_data.total_size() as u64).pack())
            .lock(lock_script.clone())
            .type_(Some(cluster_script.clone()).pack())
            .build(), Bytes::new()
    );

    let cluster_dep =  CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();

    let cluster_input = CellInput::new_builder().previous_output(cluster_out_point).build();


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

    let mut cell_deps = Vec::new();

    for dep in [lock_script_dep, cluster_script_dep, nft_script_dep, cluster_dep] {
        cell_deps.push(dep)
    }

    let output = CellOutput::new_builder()
        .capacity((output_ckb + cluster_data.total_size() as u64).pack())
        .lock(lock_script.clone())
        .type_(Some(nft_script.clone()).pack())
        .build();

    let cluster_output = CellOutput::new_builder()
        .capacity(input_ckb.pack())
        .lock(lock_script.clone())
        .type_(Some(cluster_script.clone()).pack())
        .build();

    let tx = TransactionBuilder::default()
        .inputs(vec![input, cluster_input])
        .outputs(vec![output, cluster_output])
        .output_data(serialized.as_slice().pack())
        .output_data(cluster_data.as_slice().pack())

        .set_cell_deps(cell_deps)
        .build();

    println!("data: {:?}", hex::encode(serialized.as_slice()));

    (context, tx)
}



fn build_create_context(nft_content: Vec<u8>, nft_type: String) -> (Context, TransactionView) {
    let nft_data: NativeNFTData = NativeNFTData {
        content: nft_content.clone(),
        content_type: nft_type.clone(),
        cluster: None
    };

    let serialized = SporeData::from(nft_data);
    let mut context = Context::default();
    let nft_bin: Bytes = Loader::default().load_binary("spore");
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

    println!("data: {:?}", hex::encode(serialized.as_slice()));

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
fn test_simple_with_cluster() {
    let (mut context, tx) =
        build_create_context_with_cluster(
            "".as_bytes().to_vec(),
            "plain/text".to_string(),
            "0x15561186".to_string()
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
    context
        .verify_tx(&tx, MAX_CYCLES).expect("Empty Content");
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
    let hex_str = "42020000100000001e000000420200000a000000696d6167652f6a706567200200002f396a2f34414151536b5a4a5267414241514541534142494141442f3277424441416f484277674842676f494341674c43676f4c446867514467304e44683056466845594978386c4a4349664969456d4b7a63764a696b304b5345694d4545784e446b37506a342b4a53354553554d3853446339506a762f3277424441516f4c4377344e44687751454277374b43496f4f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a762f774141524341414b41416f444153494141684542417845422f3851414651414241514141414141414141414141414141414141414241662f7841416645414144414149434177454241414141414141414141414241674d4542514152426945785152542f784141554151454141414141414141414141414141414141414141412f38514146424542414141414141414141414141414141414141414141502f61414177444151414345514d52414438415a486337584331766d327774733878386473375934556d6537482b4e305174416f53657071537a7036396c6a454166655550784f31636e773753337656363172723450536a73575a324d314a4a4a2b6b6e393470394e71715975526976724d4e73664b71625a456a42536c6e4a424c4d4f756d627341396e33364845786a4c47684f454a4a4b556c43546d69685652514f6741423841483577502f5a";
    let data = decode_hex(hex_str).unwrap();
    let nft = SporeData::from_slice(data.as_slice()).expect("error parse");

    println!("content-type: {:?}, content: {:?}, cluster: {:?}", nft.content_type(), nft.content(), nft.cluster());
}

#[test]
fn test_error_data() {
    let data: Vec<u8> = vec![0,0,0,0,0];
    let (mut context, tx) =
        build_create_context(
            data,
            "plain/text".to_string(),
        );

    let tx = context.complete_tx(tx);

     context.verify_tx(&tx, MAX_CYCLES).expect("Error Data");
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

        let result = context
            .verify_tx(&tx, MAX_CYCLES).expect("Error type");
    }

}