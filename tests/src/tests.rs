use std::hash::Hash;
use std::num::ParseIntError;

use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_error::Error;
use ckb_testtool::ckb_hash::{Blake2bBuilder, new_blake2b};
use ckb_testtool::ckb_traits::CellDataProvider;
use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, core::TransactionView, H256, packed::*, packed, prelude::*};
use ckb_testtool::ckb_types::core::cell::ResolvedDep::Cell;
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::context::Context;
use hex;
use hex::encode;

use spore_types::NativeNFTData;
use spore_types::generated::spore_types::{ClusterData, SporeData};
use spore_utils::calc_type_id;

use super::*;

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


#[test]
fn echo_contract_hash() {
    let contracts = vec![
        "spore",
        "cluster",
        "cluster_proxy",
        "cluster_agent",
        "spore_extension_lua",
    ];
    let mut context = Context::default();

    println!("Contract binary hashes:");
    for contract in contracts {
        let bin = Loader::default().load_binary(contract);
        let outpoint = context.deploy_cell(bin);
        let hash = context.get_cell_data_hash(&outpoint).unwrap_or_default().unpack();
        println!("{}, {:?}", contract, hash);
    }

}

#[test]
fn test_type_id() {
    let tx_input_outputs: Vec<(&str, usize, usize)> = vec![
        (
            "3000eab35317a9571da21522113ee60fdafbb70eaf833d6e5278047441aa3a39",
            0x1,
            0x0
        ),
        (
            "174d49d39754b2147bed7b09375b4c746436ee66261de012ecb34ca88a8841a3",
            0x0,
            0x1
        ),
        (
            "bfb080af1de0c066318766ca76433a2abcffbd5dfb6d8d9c79fe9e87dbdadb90",
            0x1,
            0x2
        ),
        (
            "0da1d47084fda2eb66ebf744c8afa916c9a633df9b0a5a6ebe600400f8c58311",
            0x1,
            0x0
        ),
    ];

    let type_id_should_be = vec![
        "9b922def4aa6fb86836673896b4b59bd7ee2bb703cfde42ea1326d662a524bf7",
        "a8a85678062badbca7580732b77b117337ce3944f5ea09d35d281ea4c6ff2fc2",
        "6143e89162ff5eb2a4d4272e35afd05ade4ed625a22686d2a87f9bc323ac1c2a",
        "47a9baeffda95b95fe3c413308ad34af41ab4cbeecf7c6b3db1af91d8c5c6156",
    ];


    tx_input_outputs.into_iter().enumerate().for_each(
        |(index, (tx_hash,
            in_output_index, out_index))| {
            let hash_raw = hex::decode(
                tx_hash.trim_end()
            ).expect("Failed to parse tx hash string!");
            let packed_data = CellInput::new_builder()
                .since(Uint64::default())
                .previous_output(
                    OutPoint::new_builder()
                        .tx_hash(Byte32::from_slice(hash_raw.as_slice()).expect("Parse to byte32"))
                        .index(in_output_index.pack())
                        .build()
                )
                .build();
            let wanted_id = H256::from_trimmed_str(type_id_should_be[index]).expect("Failed decode type_id");
            let target_id = calc_type_id(packed_data.as_slice(), out_index);
            if wanted_id.as_bytes()[..] != target_id[..] {
                panic!("Veiry type_id:\nexpect:\t{:?}\ngot:\t{:?}", wanted_id.as_bytes(), target_id);
            }
        });
}

#[test]
fn test_simple_spore_mint() {
    let spore_content: Vec<u8> = "Hello Spore!".as_bytes().to_vec();
    let spore_type = String::from("plain/text");
    let spore_data: NativeNFTData = NativeNFTData {
        content: spore_content.clone(),
        content_type: spore_type.clone(),
        cluster_id: None,
    };
    let serialized = SporeData::from(spore_data);

    let capacity = serialized.total_size() as u64;

    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();


    let input_cell = build_normal_input(&mut context, capacity, lock.clone());

    println!("input cell hash: {:?}, out_index: {}", input_cell.previous_output().tx_hash().unpack().to_string(), 0);
    let spore_type_id = build_script_args(&input_cell, 0);
    let type_ = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, type_.clone(), lock.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(spore_out_cell)
        .output_data(serialized.as_slice().pack())
        .cell_dep(spore_script_dep).build();

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore mint");
}

#[test]
fn test_simple_cluster_mint() {
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into()).build();

    let capacity = cluster.total_size() as u64;

    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder().out_point(cluster_out_point.clone()).build();
    let input_cell = build_normal_input(&mut context, capacity, lock.clone());
    let cluster_type_id = build_script_args(&input_cell, 0);
    let type_ = build_script(&mut context, &cluster_out_point, ScriptHashType::Data1, cluster_type_id.clone());
    let cluster_out_cell = build_output_cell_with_type_id(&mut context, capacity, type_.clone(), lock.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(cluster_out_cell)
        .output_data(cluster.as_slice().pack())
        .cell_dep(cluster_script_dep).build();
    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore mint");
}

#[test]
fn test_simple_spore_mint_with_cluster() {
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into()).build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder().out_point(cluster_out_point.clone()).build();
    let input_cell = build_normal_input(&mut context, cluster_capacity, lock.clone());
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type = build_script(&mut context, &cluster_out_point, ScriptHashType::Data1, cluster_type_id.clone());
    let cluster_input_cell = build_cluster_input(&mut context, cluster.clone(), cluster_type.clone(), lock.clone());
    let cluster_output_cell = build_output_cell_with_type_id(&mut context, cluster_capacity, cluster_type.clone(), lock.clone());

    // spore
    let spore_content: Vec<u8> = "Hello Spore!".as_bytes().to_vec();
    let spore_type = String::from("plain/text");
    let spore_data: NativeNFTData = NativeNFTData {
        content: spore_content.clone(),
        content_type: spore_type.clone(),
        cluster_id: Some(cluster_type_id.to_vec().clone()),
    };
    let serialized = SporeData::from(spore_data);
    let capacity = serialized.total_size() as u64;
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();
    let spore_cluster_dep = CellDep::new_builder().out_point(cluster_input_cell.previous_output()).build();

    let input_cell = build_normal_input(&mut context, capacity, lock.clone());
    let spore_type_id = build_script_args(&input_cell, 1);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());
    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone(), lock.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell, cluster_input_cell])
        .outputs(vec![cluster_output_cell, spore_out_cell])
        .outputs_data(vec![cluster.as_slice().pack(), serialized.as_slice().pack()])
        .cell_deps(vec![cluster_script_dep, spore_cluster_dep, spore_script_dep]).build();
    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore mint");
}

#[test]
fn test_spore_mint_with_lock_proxy() {
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into()).build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder().out_point(cluster_out_point.clone()).build();
    let input_cell = build_normal_input(&mut context, cluster_capacity, lock.clone());
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type = build_script(&mut context, &cluster_out_point, ScriptHashType::Data1, cluster_type_id.clone());
    let cluster_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((cluster.total_size() as u64).pack())
            .lock(lock.clone())
            .type_(ScriptOpt::new_builder().set(cluster_type.clone()).build())
            .build(), Bytes::copy_from_slice(cluster.as_slice()),
    );
    let cluster_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();

    // spore
    let spore_content: Vec<u8> = "Hello Spore!".as_bytes().to_vec();
    let spore_type = String::from("plain/text");
    let spore_data: NativeNFTData = NativeNFTData {
        content: spore_content.clone(),
        content_type: spore_type.clone(),
        cluster_id: Some(cluster_type_id.to_vec().clone()),
    };
    let serialized = SporeData::from(spore_data);
    let capacity = serialized.total_size() as u64;
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();

    let input_cell = build_normal_input(&mut context, capacity, lock.clone());
    let spore_type_id = build_script_args(&input_cell, 0);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());
    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone(), lock.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell])
        .outputs(vec![spore_out_cell])
        .outputs_data(vec![serialized.as_slice().pack()])
        .cell_deps(vec![cluster_script_dep, spore_script_dep, cluster_dep]).build();
    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test spore mint with lock proxy");
}

#[test]
fn test_spore_mint_with_lock_proxy_failure() {
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let lock2 = build_always_success_script(&mut context);

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into()).build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder().out_point(cluster_out_point.clone()).build();
    let input_cell = build_normal_input(&mut context, cluster_capacity, lock.clone());
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type = build_script(&mut context, &cluster_out_point, ScriptHashType::Data1, cluster_type_id.clone());
    let cluster_cell = build_cluster_input(&mut context, cluster.clone(), cluster_type.clone(), lock.clone());
    // spore
    let spore_content: Vec<u8> = "Hello Spore!".as_bytes().to_vec();
    let spore_type = String::from("plain/text");
    let spore_data: NativeNFTData = NativeNFTData {
        content: spore_content.clone(),
        content_type: spore_type.clone(),
        cluster_id: Some(cluster_type_id.to_vec().clone()),
    };
    let serialized = SporeData::from(spore_data);
    let capacity = serialized.total_size() as u64;
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();
    let spore_cluster_dep = CellDep::new_builder().out_point(cluster_cell.previous_output()).build();

    let input_cell = build_normal_input(&mut context, capacity, lock2.clone());
    let spore_type_id = build_script_args(&input_cell, 0);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());
    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone(), lock.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell])
        .outputs(vec![spore_out_cell])
        .outputs_data(vec![cluster.as_slice().pack(), serialized.as_slice().pack()])
        .cell_deps(vec![cluster_script_dep, spore_cluster_dep, spore_script_dep]).build();
    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect_err("test spore mint with lock proxy failure case");
}


fn build_simple_tx(input_cells: Vec<CellInput>, output_cells: Vec<CellOutput>, cell_deps: Vec<CellDep>, outputs_data: Vec<packed::Bytes>) -> TransactionView {
    TransactionBuilder::default()
        .inputs(input_cells)
        .outputs(output_cells)
        .outputs_data(outputs_data)
        .cell_deps(cell_deps)
        .build()
}

fn build_spore_materials(context: &mut Context) -> (OutPoint, CellDep) {
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();
    (spore_out_point, spore_script_dep)
}

#[test]
fn test_simple_spore_transfer() {
    let serialized = build_serialized_spore("Hello Spore!".as_bytes().to_vec(), "plain/text".to_string());
    let capacity = serialized.total_size() as u64;
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let spore_type_id = build_script_args(&build_normal_input(&mut context, capacity, lock.clone()), 0);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());
    let spore_input = build_spore_input(&mut context, &spore_out_point, serialized.clone(), spore_type_id.clone(), lock.clone());

    let spore_output = build_output_cell_with_type_id(&mut context, capacity, spore_type, lock.clone());
    let tx = build_simple_tx(
        vec![spore_input],
        vec![spore_output],
        vec![spore_script_dep],
        vec![serialized.as_slice().pack()],
    );

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore transfer");
}

fn build_serialized_spore(nft_content: Vec<u8>, nft_type: String) -> SporeData {
    SporeData::from(NativeNFTData {
        content: nft_content.clone(),
        content_type: nft_type.clone(),
        cluster_id: None,
    })
}

fn simple_build_context(output_data: Vec<u8>, content_type: String, input_data: Option<SporeData>, out_index: usize) -> (Context, TransactionView) {
    let output_data = build_serialized_spore(output_data, content_type);
    let capacity = output_data.total_size() as u64;
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let (input, type_id) = match input_data {
        None => {
            let input = build_normal_input(&mut context, capacity, lock.clone());
            let spore_type_id = build_script_args(&input, out_index);
            (input, spore_type_id)
        }
        Some(input_data) => {
            let input_capacity = input_data.total_size() as u64;
            let spore_type_id = build_script_args(&build_normal_input(&mut context, input_capacity, lock.clone()), out_index);
            let spore_input = build_spore_input(&mut context, &spore_out_point, input_data, spore_type_id.clone(), lock.clone());
            (spore_input, spore_type_id)
        }
    };
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, type_id.clone());
    let spore_output = build_output_cell_with_type_id(&mut context, capacity, spore_type, lock.clone());
    let tx = build_simple_tx(
        vec![input],
        vec![spore_output],
        vec![spore_script_dep],
        vec![output_data.as_slice().pack()],
    );

    (context, tx)
}

#[test]
fn test_simple_spore_mint2() {
    let (mut context, tx) =
        simple_build_context("THIS IS A TEST NFT".as_bytes().to_vec(),
                             "plain/text".to_string(),
                             None,
                             0,
        );
    let tx = context.complete_tx(tx);
    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore mint 2");
}

#[test]
fn test_simple_spore_mint3() { // mint with normal output tx test
    let spore_content: Vec<u8> = "Hello Spore!".as_bytes().to_vec();
    let spore_type = String::from("plain/text");
    let spore_data: NativeNFTData = NativeNFTData {
        content: spore_content.clone(),
        content_type: spore_type.clone(),
        cluster_id: None,
    };
    let serialized = SporeData::from(spore_data);

    let capacity = serialized.total_size() as u64;

    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();


    let input_cell1 = build_normal_input(&mut context, capacity, lock.clone());
    let input_cell2 = build_normal_input(&mut context, capacity, lock.clone());
    let input_cell3 = build_normal_input(&mut context, capacity, lock.clone());

    let spore_type_id = build_script_args(&input_cell1, 0);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone(), lock.clone());
    let output_cell1 = build_normal_output(&mut context, capacity, lock.clone());


    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell1, input_cell2, input_cell3])
        .outputs(vec![spore_out_cell, output_cell1])
        .outputs_data(vec![serialized.as_slice().pack(), packed::Bytes::default()])
        .cell_dep(spore_script_dep).build();

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore mint 3");
}


#[test]
fn test_simple_spore_mint4() { // multiple mint tx test
    let spore_content: Vec<u8> = "Hello Spore!".as_bytes().to_vec();
    let spore_type = String::from("plain/text");
    let spore_data: NativeNFTData = NativeNFTData {
        content: spore_content.clone(),
        content_type: spore_type.clone(),
        cluster_id: None,
    };
    let serialized = SporeData::from(spore_data);

    let capacity = serialized.total_size() as u64;

    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder().out_point(spore_out_point.clone()).build();


    let input_cell1 = build_normal_input(&mut context, capacity, lock.clone());
    let input_cell2 = build_normal_input(&mut context, capacity, lock.clone());
    let input_cell3 = build_normal_input(&mut context, capacity, lock.clone());

    let spore_type_id = build_script_args(&input_cell1, 0);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone(), lock.clone());
    let output_cell1 = build_normal_output(&mut context, capacity, lock.clone());

    let spore_type_id2 = build_script_args(&input_cell1, 2);
    let spore_type2 = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id2.clone());

    let spore_out_cell2 = build_output_cell_with_type_id(&mut context, capacity, spore_type2.clone(), lock.clone());


    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell1, input_cell2, input_cell3])
        .outputs(vec![spore_out_cell, output_cell1, spore_out_cell2])
        .outputs_data(vec![serialized.as_slice().pack(), packed::Bytes::default(), serialized.as_slice().pack()])
        .cell_dep(spore_script_dep).build();

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test simple spore mint 3");
}


#[test]
fn test_spore_multipart_mint() {
    let (mut context, tx) =
        simple_build_context("THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include".as_bytes().to_vec(),
                             "multipart/mixed;boundary=SporeDefaultBoundary".to_string(),
                             None,
                             0,
        );
    let tx = context.complete_tx(tx);
    context.verify_tx(&tx, MAX_CYCLES).expect("test multipart mint");
}

#[test]
fn test_spore_multipart_mint_failure01() {
    let (mut context, tx) =
        simple_build_context("THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include".as_bytes().to_vec(),
                             "multipart/mixed;".to_string(), // no boundary param
                             None,
                             0,
        );
    let tx = context.complete_tx(tx);
    context.verify_tx(&tx, MAX_CYCLES).expect_err("test multipart failure 01");
}

#[test]
fn test_spore_multipart_mint_failure02() {
    let (mut context, tx) =
        simple_build_context("THIS IS A TEST MULTIPART NFT\n\nThis is an extra message I want to include".as_bytes().to_vec(),
                             "multipart/mixed;boundary=SporeDefaultBoundary;".to_string(),
                             None,
                             0,
        );
    let tx = context.complete_tx(tx);
    context.verify_tx(&tx, MAX_CYCLES).expect_err("test multipart failure 02");
}

#[test]
fn test_simple_with_cluster() {
    let (mut context, tx) =
        build_simple_create_context_with_cluster(
            "THIS IS A SIMPLE SPORE".to_string(),
            "plain/text".to_string(),
            "0x12345678".to_string(),
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
        simple_build_context(
            vec![],
            "plain/text".to_string(),
            None,
            0,
        );
    let tx = context.complete_tx(tx);

    // run
    context
        .verify_tx(&tx, MAX_CYCLES).expect_err("Empty Content");
}

#[test]
fn test_read_file() {
    use std::io::Read;
    use std::io::BufReader;
    use std::fs::File;
    let f = File::open("res/test.jpg");
    if f.is_ok() {
        let f = f.unwrap();
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).expect("Error read to end");

        simple_build_context(buffer, "image/jpeg".to_string(), None, 0);
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
    let (mut context, tx) =
        simple_build_context("/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAoHBwgHBgoICAgLCgoLDhgQDg0NDh0VFhEYIx8lJCIfIiEmKzcvJik0KSEiMEExNDk7Pj4+JS5ESUM8SDc9Pjv/2wBDAQoLCw4NDhwQEBw7KCIoOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozv/wAARCAAKAAoDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAABAf/xAAfEAADAAICAwEBAAAAAAAAAAABAgMEBQARBiExQRT/xAAUAQEAAAAAAAAAAAAAAAAAAAAA/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAwDAQACEQMRAD8AZHc7XC1vm2wts8x8ds7Y4Ume7H+N0QtAoSepqSzp69ljEAfeUPxO1cnw7S3vV61rr4PSjsWZ2M1JJJ+kn94p9NqqYuRivrMNsfKqbZEjBSlnJBLMOumbsA9n36HExjLGhOEJJKUlCTmihVRQOgAB8AH5wP/Z".as_bytes().to_vec(),
                             "image/jpeg".to_string(),
                             None,
                             0);
    let view = context.complete_tx(tx);
    context.verify_tx(&view, MAX_CYCLES).expect("Error tx");
}

#[test]
fn test_decode_hex() {
    let hex_str = "42020000100000001e000000420200000a000000696d6167652f6a706567200200002f396a2f34414151536b5a4a5267414241514541534142494141442f3277424441416f484277674842676f494341674c43676f4c446867514467304e44683056466845594978386c4a4349664969456d4b7a63764a696b304b5345694d4545784e446b37506a342b4a53354553554d3853446339506a762f3277424441516f4c4377344e44687751454277374b43496f4f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a73374f7a762f774141524341414b41416f444153494141684542417845422f3851414651414241514141414141414141414141414141414141414241662f7841416645414144414149434177454241414141414141414141414241674d4542514152426945785152542f784141554151454141414141414141414141414141414141414141412f38514146424542414141414141414141414141414141414141414141502f61414177444151414345514d52414438415a486337584331766d327774733878386473375934556d6537482b4e305174416f53657071537a7036396c6a454166655550784f31636e773753337656363172723450536a73575a324d314a4a4a2b6b6e393470394e71715975526976724d4e73664b71625a456a42536c6e4a424c4d4f756d627341396e33364845786a4c47684f454a4a4b556c43546d69685652514f6741423841483577502f5a";
    let data = decode_hex(hex_str).unwrap();
    let nft = SporeData::from_slice(data.as_slice()).expect("error parse");

    println!("content-type: {:?}, content: {:?}, cluster: {:?}", nft.content_type(), nft.content(), nft.cluster_id());
}

#[test]
fn test_error_data() {
    let data: Vec<u8> = vec![0, 0, 0, 0, 0];
    let (mut context, tx) =
        simple_build_context(
            data,
            "plain/text".to_string(),
            None,
            0,
        );

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("Error Data");
}

#[test]
fn test_destroy() {
    let serialized = build_serialized_spore("Hello Spore!".as_bytes().to_vec(), "plain/text".to_string());
    let capacity = serialized.total_size() as u64;
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let spore_type_id = build_script_args(&build_normal_input(&mut context, capacity, lock.clone()), 0);
    let spore_input = build_spore_input(&mut context, &spore_out_point, serialized.clone(), spore_type_id.clone(), lock.clone());

    let output = build_normal_output(&mut context, capacity, lock.clone());
    let tx = build_simple_tx(
        vec![spore_input],
        vec![output],
        vec![spore_script_dep],
        vec![serialized.as_slice().pack()],
    );

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("try destroy immortal");
}

#[test]
fn test_destroy_immortal() {
    let serialized = build_serialized_spore("Hello Spore!".as_bytes().to_vec(), "plain/text;immortal=true".to_string());
    let capacity = serialized.total_size() as u64;
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let spore_type_id = build_script_args(&build_normal_input(&mut context, capacity, lock.clone()), 0);
    let spore_type = build_script(&mut context, &spore_out_point, ScriptHashType::Data1, spore_type_id.clone());
    let spore_input = build_spore_input(&mut context, &spore_out_point, serialized.clone(), spore_type_id.clone(), lock.clone());

    let output = build_normal_output(&mut context, capacity, lock.clone());
    let tx = build_simple_tx(
        vec![spore_input],
        vec![output],
        vec![spore_script_dep],
        vec![packed::Bytes::default()],
    );

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect_err("try destroy immortal");
}

#[test]
fn test_destroy_cluster() {
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into()).build();

    let capacity = cluster.total_size() as u64;



    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder().out_point(cluster_out_point.clone()).build();
    let cluster_type_id = build_script_args(&build_normal_input(&mut context, capacity, lock.clone()), 0);
    let type_ = build_script(&mut context, &cluster_out_point, ScriptHashType::Data1, cluster_type_id.clone());

    let cluster_input = build_cluster_input(&mut context, cluster, type_.clone(), lock.clone());

    let output_cell = build_normal_output(&mut context, capacity, lock.clone());

    let tx = TransactionBuilder::default()
        .input(cluster_input)
        .output(output_cell)
        .output_data(packed::Bytes::default())
        .cell_dep(cluster_script_dep).build();
    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect_err("test destroy cluster");
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
            simple_build_context(
                "THIS IS A TEST NFT".as_bytes().to_vec(),
                content_type.to_string(),
                None,
                0,
            );
        let tx = context.complete_tx(tx);

        let result = context
            .verify_tx(&tx, MAX_CYCLES).expect_err("Error type");
    }
}

fn calc_code_hash(data: Bytes) -> [u8; 32] {
    let mut blake2b = new_blake2b();
    let mut buf = [0u8; 8 * 1024];
    blake2b.update(data.to_vec().as_slice());
    let mut hash = [0u8; 32];
    blake2b.finalize(&mut hash);
    hash
}

#[test]
fn test_code_hash() {
    let binary_list = vec![
        "spore", "cluster", "cluster_agent", "cluster_proxy", "spore_extension_lua", "libckblua.so"
    ];
    for lib in binary_list {
        let bin: Bytes = Loader::default().load_binary(lib);
        let code_hash = CellOutput::calc_data_hash(&bin.clone());
        println!("{} code_hash: 0x{}, to vec: {:?}", lib, encode(code_hash.as_slice()), code_hash.as_slice().to_vec());
    }
}

#[test]
fn test_extension_1() {
    let mut context = Context::default();
    // always success lock
    let lock = build_always_success_script(&mut context);
    let lua_lib_bin: Bytes = Loader::default().load_binary("libckblua.so");
    let lua_lib_out_point = context.deploy_cell(lua_lib_bin.clone());
    let code_hash = CellOutput::calc_data_hash(&lua_lib_bin.clone());
    let lua_lib_dep = CellDep::new_builder().out_point(lua_lib_out_point.clone()).build();

    println!("lua lib hash: {}", encode(code_hash.as_slice()));

    let spore_extension_bin: Bytes = Loader::default().load_binary("spore_extension_lua");
    println!("extension hash: {}", encode(calc_code_hash(spore_extension_bin.clone())));
    let spore_extension_out_point = context.deploy_cell(spore_extension_bin);
    let spore_extension_script_dep = CellDep::new_builder().out_point(spore_extension_out_point.clone()).build();

    let lua_code = String::from("print(\"hello\")");

    let capacity = lua_code.len() as u64;

    let input_cell = build_normal_input(&mut context, capacity, lock.clone());

    println!("input cell hash: {:?}, out_index: {}", input_cell.previous_output().tx_hash().unpack().to_string(), 0);
    let spore_extension_type_id = build_script_args(&input_cell, 0);
    let type_ = build_script(&mut context, &spore_extension_out_point, ScriptHashType::Data1, spore_extension_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, type_.clone(), lock.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(spore_out_cell)
        .output_data(lua_code.pack())
        .cell_deps(vec![lua_lib_dep, spore_extension_script_dep])
        .build();

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test spore lua extension");
}


fn build_simple_create_context_with_cluster(nft_content: String, nft_type: String, cluster_id: String) -> (Context, TransactionView) {
    let nft_data: NativeNFTData = NativeNFTData {
        content: nft_content.clone().into_bytes(),
        content_type: nft_type.clone(),
        cluster_id: Some(H256::from_trimmed_str(cluster_id.clone().trim_start_matches("0x")).expect("parse cluster id").as_bytes().to_vec()),
    };
    let serialized = SporeData::from(nft_data);
    build_create_context_with_cluster_raw(serialized, cluster_id)
}

fn build_create_context_with_cluster_raw(nft_data: SporeData, cluster_id: String) -> (Context, TransactionView) {
    let dummy_cluster_name = "Spore Cluster!";
    let dummy_cluster_description = "Spore Description!";


    let cluster_data = ClusterData::new_builder()
        .name(dummy_cluster_name.pack().as_slice().into())
        .description(dummy_cluster_description.pack().as_slice().into())
        .build();
    let mut context = Context::default();
    let nft_bin: Bytes = Loader::default().load_binary("spore");
    let nft_out_point = context.deploy_cell(nft_bin);
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let input_ckb = { nft_data.total_size() } as u64;

    let output_ckb = input_ckb;
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

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
            .build(), Bytes::new(),
    );

    let cluster_id = H256::from_trimmed_str(cluster_id.clone().trim_start_matches("0x")).expect("parse cluster id").pack();

    let cluster_script = context
        .build_script_with_hash_type(
            &cluster_out_point,
            ScriptHashType::Data1,
            cluster_id.raw_data(),
        )
        .expect("cluster script");


    let cluster_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((cluster_data.total_size() as u64).pack())
            .lock(lock_script.clone())
            .type_(Some(cluster_script.clone()).pack())
            .build(), Bytes::copy_from_slice(cluster_data.as_slice()),
    );

    let cluster_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();

    let cluster_input = CellInput::new_builder()
        .previous_output(cluster_out_point)
        .build();

    let normal_input = CellInput::new_builder()
        .previous_output(
            context.create_cell(CellOutput::new_builder()
                                    .capacity((1000000 as u64).pack())
                                    .lock(lock_script.clone())
                                    .build(), Bytes::new())
        ).build();


    let input = CellInput::new_builder().previous_output(input_out_point).build();


    let nft_script_args: Bytes = {
        let mut blake2b = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        blake2b.update(input.as_slice());
        blake2b.update(&(1 as u64).to_le_bytes());
        let mut verify_id = [0; 32];
        blake2b.finalize(&mut verify_id);
        verify_id.to_vec().into()
    };


    let nft_script = context
        .build_script_with_hash_type(&nft_out_point, ScriptHashType::Data1, nft_script_args)
        .expect("script");

    let nft_script_dep = CellDep::new_builder().out_point(nft_out_point).build();


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

    let normal_output = CellOutput::new_builder()
        .capacity(9999u64.pack())
        .lock(lock_script.clone())
        .build();

    let tx = TransactionBuilder::default()
        .inputs(vec![input, normal_input, cluster_input])
        .outputs(vec![normal_output, output, cluster_output])
        .outputs_data(vec![packed::Bytes::default(), nft_data.as_slice().pack(), cluster_data.as_slice().pack()])
        .cell_deps(vec![lock_script_dep, cluster_script_dep, nft_script_dep, cluster_dep])
        .build();

    println!("data: {:?}", hex::encode(nft_data.as_slice()));

    (context, tx)
}

fn build_outpoint(context: &mut Context, capacity: u64, type_script: Option<Script>, lock_script: Script, data: Bytes) -> OutPoint {
    context.create_cell(
        build_output(capacity, type_script, lock_script), data,
    )
}

fn build_mock_outpoint(tx_hash: Byte32, index: usize) -> OutPoint {
    OutPoint::new_builder()
        .tx_hash(tx_hash)
        .index((index as u32).pack())
        .build()
}

fn build_input(context: &mut Context, capacity: u64, type_script: Option<Script>, lock_script: Script, data: Bytes) -> CellInput {
    let outpoint = build_outpoint(context, capacity, type_script, lock_script, data);
    CellInput::new_builder()
        .since(Uint64::default())
        .previous_output(outpoint)
        .build()
}

fn build_output(capacity: u64, type_script: Option<Script>, lock_script: Script) -> CellOutput {
    CellOutput::new_builder()
        .capacity(capacity.pack())
        .lock(lock_script.clone())
        .type_(ScriptOpt::new_builder().set(type_script).build())
        .build()
}

fn build_always_success_script(context: &mut Context) -> Script {
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // build lock script
    context
        .build_script(&always_success_out_point, Default::default())
        .expect("always success script")
}

fn build_script_args(first_input: &CellInput, out_index: usize) -> Bytes {
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(first_input.as_slice());
    blake2b.update(&(out_index).to_le_bytes());
    let mut verify_id = [0; 32];
    blake2b.finalize(&mut verify_id);
    Bytes::from(verify_id.to_vec())
}

fn build_script(context: &mut Context, outpoint: &OutPoint, hash_type: ScriptHashType, args: Bytes) -> Option<Script> {
    context.build_script_with_hash_type(outpoint, hash_type, args)
}

fn build_spore_type_script(context: &mut Context, spore_out_point: &OutPoint, args: Bytes) -> Option<Script> {
    build_script(context, spore_out_point, ScriptHashType::Data1, args)
}

fn build_spore_input(context: &mut Context, spore_out_point: &OutPoint, spore_data: SporeData, type_id: Bytes, lock: Script) -> CellInput {
    let input_ckb = spore_data.total_size() as u64;
    let type_ = build_spore_type_script(context, &spore_out_point, type_id);
    build_input(context,
                input_ckb,
                type_,
                lock,
                Bytes::copy_from_slice(
                    spore_data.as_slice()
                ),
    )
}

fn build_cluster_input(context: &mut Context, cluster_data: ClusterData, type_: Option<Script>, lock: Script) -> CellInput {
    let input_ckb = cluster_data.total_size() as u64;
    build_input(context,
                input_ckb,
                type_,
                lock,
                Bytes::copy_from_slice(
                    cluster_data.as_slice()
                ),
    )
}

fn build_normal_input(context: &mut Context, capacity: u64, lock_script: Script) -> CellInput {
    build_input(context, capacity, None, lock_script, Bytes::new())
}

fn build_output_cell_with_type_id(context: &mut Context, capacity: u64, type_: Option<Script>, lock: Script) -> CellOutput {
    build_output(capacity,
                 type_,
                 lock,
    )
}

fn build_normal_output(context: &mut Context, capasity: u64, lock: Script) -> CellOutput {
    build_output(capasity, None, lock)
}
