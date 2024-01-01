use ckb_testtool::ckb_types::{
    bytes::Bytes, core::TransactionBuilder, packed, packed::*, prelude::*,
};
use ckb_testtool::context::Context;
use hex;
use hex::encode;

use spore_types::generated::spore_types::{ClusterData, SporeData};
use spore_types::NativeNFTData;

use crate::utils::*;
use crate::Loader;

const MAX_CYCLES: u64 = 10_000_000;

#[test]
fn test_simple_spore_mint() {
    let serialized = build_serialized_spore("Hello Spore!", "plain/text");
    let capacity = serialized.total_size() as u64;

    let mut context = Context::default();

    // always success lock
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder()
        .out_point(spore_out_point.clone())
        .build();

    let input_cell = build_normal_input(&mut context, capacity);

    println!(
        "input cell hash: {:?}, out_index: {}",
        input_cell.previous_output().tx_hash().unpack().to_string(),
        0
    );
    let spore_type_id = build_script_args(&input_cell, 0);
    let type_ = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, type_.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(spore_out_cell)
        .output_data(serialized.as_slice().pack())
        .cell_dep(spore_script_dep)
        .build();

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint");
}

#[test]
fn test_simple_cluster_mint() {
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();
    let capacity = cluster.total_size() as u64;

    let mut context = Context::default();

    // always success lock
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let input_cell = build_normal_input(&mut context, capacity);
    let cluster_type_id = build_script_args(&input_cell, 0);
    let type_ = build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());
    let cluster_out_cell = build_output_cell_with_type_id(&mut context, capacity, type_.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(cluster_out_cell)
        .output_data(cluster.as_slice().pack())
        .cell_dep(cluster_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint");
}

#[test]
fn test_simple_spore_mint_with_cluster() {
    let mut context = Context::default();

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let input_cell = build_normal_input(&mut context, cluster_capacity);
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());
    let cluster_input_cell =
        build_cluster_input(&mut context, cluster.clone(), cluster_type.clone());
    let cluster_output_cell =
        build_output_cell_with_type_id(&mut context, cluster_capacity, cluster_type.clone());

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
    let spore_script_dep = CellDep::new_builder()
        .out_point(spore_out_point.clone())
        .build();
    let spore_cluster_dep = CellDep::new_builder()
        .out_point(cluster_input_cell.previous_output())
        .build();

    let input_cell = build_normal_input(&mut context, capacity);
    let spore_type_id = build_script_args(&input_cell, 1);
    let spore_type = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());
    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell, cluster_input_cell])
        .outputs(vec![cluster_output_cell, spore_out_cell])
        .outputs_data(vec![
            cluster.as_slice().pack(),
            serialized.as_slice().pack(),
        ])
        .cell_deps(vec![
            cluster_script_dep,
            spore_cluster_dep,
            spore_script_dep,
        ])
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint");
}

#[test]
fn test_spore_mint_with_lock_proxy() {
    let mut context = Context::default();

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let input_cell = build_normal_input(&mut context, cluster_capacity);
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());
    let cluster_dep = build_normal_cell_dep(&mut context, cluster.as_slice(), cluster_type);

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
    let spore_script_dep = CellDep::new_builder()
        .out_point(spore_out_point.clone())
        .build();

    let input_cell = build_normal_input(&mut context, capacity);
    let spore_type_id = build_script_args(&input_cell, 0);
    let spore_type = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());
    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell])
        .outputs(vec![spore_out_cell])
        .outputs_data(vec![serialized.as_slice().pack()])
        .cell_deps(vec![cluster_script_dep, spore_script_dep, cluster_dep])
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore mint with lock proxy");
}

#[test]
fn test_spore_mint_with_lock_proxy_failure() {
    let mut context = Context::default();

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let input_cell = build_normal_input(&mut context, cluster_capacity);
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());
    let cluster_cell = build_cluster_input(&mut context, cluster.clone(), cluster_type.clone());
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
    let spore_script_dep = CellDep::new_builder()
        .out_point(spore_out_point.clone())
        .build();
    let spore_cluster_dep = CellDep::new_builder()
        .out_point(cluster_cell.previous_output())
        .build();

    let input_cell = build_normal_input(&mut context, capacity);
    let spore_type_id = build_script_args(&input_cell, 0);
    let spore_type = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());
    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell])
        .outputs(vec![spore_out_cell])
        .outputs_data(vec![
            cluster.as_slice().pack(),
            serialized.as_slice().pack(),
        ])
        .cell_deps(vec![
            cluster_script_dep,
            spore_cluster_dep,
            spore_script_dep,
        ])
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("test spore mint with lock proxy failure case");
}

#[test]
fn test_spore_mint_with_cluster_proxy() {
    let mut context = Context::default();

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let input_cell = build_normal_input(&mut context, cluster_capacity);
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());
    let cluster_dep = build_normal_cell_dep(&mut context, cluster.as_slice(), cluster_type);

    // proxy
    let capacity = cluster_type_id.len() as u64;
    let proxy_bin: Bytes = Loader::default().load_binary("cluster_proxy");
    let proxy_out_point = context.deploy_cell(proxy_bin);
    let proxy_script_dep = CellDep::new_builder()
        .out_point(proxy_out_point.clone())
        .build();

    let input_cell = build_normal_input(&mut context, capacity);
    let proxy_type_id = build_script_args(&input_cell, 0);
    let proxy_type = build_spore_type_script(&mut context, &proxy_out_point, proxy_type_id.clone());
    let proxy_out_cell = build_output_cell_with_type_id(&mut context, capacity, proxy_type.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell])
        .outputs(vec![proxy_out_cell])
        .outputs_data(vec![cluster_type_id.pack()])
        .cell_deps(vec![cluster_script_dep, proxy_script_dep, cluster_dep])
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore mint with lock proxy");
}

#[test]
fn test_cluster_agent() {
    let mut context = Context::default();

    // cluster
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();

    let cluster_capacity = cluster.total_size() as u64;
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let input_cell = build_normal_input(&mut context, cluster_capacity);
    let cluster_type_id = build_script_args(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());
    let cluster_dep = build_normal_cell_dep(&mut context, cluster.as_slice(), cluster_type);

    // proxy
    let capacity = cluster_type_id.len() as u64;
    let proxy_bin: Bytes = Loader::default().load_binary("cluster_proxy");
    let proxy_out_point = context.deploy_cell(proxy_bin);
    let proxy_script_dep = CellDep::new_builder()
        .out_point(proxy_out_point.clone())
        .build();

    let input_cell = build_normal_input(&mut context, capacity);
    let proxy_type_id = build_script_args(&input_cell, 0);
    let mut proxy_type_arg = proxy_type_id.to_vec();
    proxy_type_arg.push(1);
    println!("Proxy_type_arg len: {}", proxy_type_arg.len());
    let proxy_type = build_spore_type_script(
        &mut context,
        &proxy_out_point,
        Bytes::copy_from_slice(proxy_type_arg.clone().as_slice()),
    );
    let proxy_dep = build_normal_cell_dep(&mut context, &cluster_type_id, proxy_type.clone());

    // agent
    let agent_capacity = capacity;
    let agent_bin: Bytes = Loader::default().load_binary("cluster_agent");
    let agent_out_point = context.deploy_cell(agent_bin);
    let agent_script_dep = CellDep::new_builder()
        .out_point(agent_out_point.clone())
        .build();

    let input_cell = build_normal_input(&mut context, agent_capacity);

    let agent_type = build_spore_type_script(&mut context, &agent_out_point, cluster_type_id);
    let agent_out_cell = build_output_cell_with_type_id(&mut context, capacity, agent_type.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell])
        .outputs(vec![agent_out_cell])
        .outputs_data(vec![proxy_type
            .unwrap_or_default()
            .calc_script_hash()
            .as_slice()
            .pack()])
        .cell_deps(vec![
            cluster_script_dep,
            proxy_script_dep,
            agent_script_dep,
            cluster_dep,
            proxy_dep,
        ])
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test cluster_agent create");
}

#[test]
fn test_simple_spore_transfer() {
    let serialized = build_serialized_spore("Hello Spore!", "plain/text");
    let capacity = serialized.total_size() as u64;
    let mut context = Context::default();

    // always success lock
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let spore_type_id = build_script_args(&build_normal_input(&mut context, capacity), 0);
    let spore_type = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());
    let spore_input = build_spore_input(
        &mut context,
        &spore_out_point,
        serialized.clone(),
        spore_type_id.clone(),
    );

    let spore_output = build_output_cell_with_type_id(&mut context, capacity, spore_type);
    let tx = build_simple_tx(
        vec![spore_input],
        vec![spore_output],
        vec![spore_script_dep],
        vec![serialized.as_slice().pack()],
    );

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore transfer");
}

#[test]
fn test_simple_spore_mint2() {
    let (mut context, tx) = simple_build_context(
        "THIS IS A TEST NFT".as_bytes().to_vec(),
        "plain/text",
        None,
        0,
    );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint 2");
}

#[test]
fn test_simple_spore_mint3() {
    // mint with normal output tx test
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
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder()
        .out_point(spore_out_point.clone())
        .build();

    let input_cell1 = build_normal_input(&mut context, capacity);
    let input_cell2 = build_normal_input(&mut context, capacity);
    let input_cell3 = build_normal_input(&mut context, capacity);

    let spore_type_id = build_script_args(&input_cell1, 0);
    let spore_type = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone());
    let output_cell1 = build_normal_output(&mut context, capacity);

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell1, input_cell2, input_cell3])
        .outputs(vec![spore_out_cell, output_cell1])
        .outputs_data(vec![serialized.as_slice().pack(), packed::Bytes::default()])
        .cell_dep(spore_script_dep)
        .build();

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint 3");
}

#[test]
fn test_simple_spore_mint4() {
    // multiple mint tx test
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
    let spore_bin: Bytes = Loader::default().load_binary("spore");
    let spore_out_point = context.deploy_cell(spore_bin);
    let spore_script_dep = CellDep::new_builder()
        .out_point(spore_out_point.clone())
        .build();

    let input_cell1 = build_normal_input(&mut context, capacity);
    let input_cell2 = build_normal_input(&mut context, capacity);
    let input_cell3 = build_normal_input(&mut context, capacity);

    let spore_type_id = build_script_args(&input_cell1, 0);
    let spore_type = build_spore_type_script(&mut context, &spore_out_point, spore_type_id.clone());

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, spore_type.clone());
    let output_cell1 = build_normal_output(&mut context, capacity);

    let spore_type_id2 = build_script_args(&input_cell1, 2);
    let spore_type2 =
        build_spore_type_script(&mut context, &spore_out_point, spore_type_id2.clone());

    let spore_out_cell2 =
        build_output_cell_with_type_id(&mut context, capacity, spore_type2.clone());

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell1, input_cell2, input_cell3])
        .outputs(vec![spore_out_cell, output_cell1, spore_out_cell2])
        .outputs_data(vec![
            serialized.as_slice().pack(),
            packed::Bytes::default(),
            serialized.as_slice().pack(),
        ])
        .cell_dep(spore_script_dep)
        .build();

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint 3");
}

#[test]
fn test_spore_multipart_mint() {
    let (mut context, tx) =
        simple_build_context("THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include".as_bytes().to_vec(),
                             "multipart/mixed;boundary=SporeDefaultBoundary",
                             None,
                             0,
        );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test multipart mint");
}

#[test]
fn test_spore_multipart_mint_failure01() {
    let (mut context, tx) =
        simple_build_context("THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include".as_bytes().to_vec(),
                             "multipart/mixed;",
                             None,
                             0,
        );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("test multipart failure 01");
}

#[test]
fn test_spore_multipart_mint_failure02() {
    let (mut context, tx) = simple_build_context(
        "THIS IS A TEST MULTIPART NFT\n\nThis is an extra message I want to include"
            .as_bytes()
            .to_vec(),
        "multipart/mixed;boundary=SporeDefaultBoundary;",
        None,
        0,
    );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("test multipart failure 02");
}

#[test]
fn test_simple_with_cluster() {
    let (mut context, tx) = build_simple_create_context_with_cluster(
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
    let (mut context, tx) = simple_build_context(vec![], "plain/text", None, 0);
    let tx = context.complete_tx(tx);

    // run
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("Empty Content");
}

#[test]
fn test_read_file() {
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    let f = File::open("res/test.jpg");
    if f.is_ok() {
        let f = f.unwrap();
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).expect("Error read to end");

        simple_build_context(buffer, "image/jpeg", None, 0);
    } else {
        println!("Error while reading file!");
    }
}

#[test]
fn test_read_base64() {
    let (mut context, tx) =
        simple_build_context("/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAoHBwgHBgoICAgLCgoLDhgQDg0NDh0VFhEYIx8lJCIfIiEmKzcvJik0KSEiMEExNDk7Pj4+JS5ESUM8SDc9Pjv/2wBDAQoLCw4NDhwQEBw7KCIoOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozv/wAARCAAKAAoDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAABAf/xAAfEAADAAICAwEBAAAAAAAAAAABAgMEBQARBiExQRT/xAAUAQEAAAAAAAAAAAAAAAAAAAAA/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAwDAQACEQMRAD8AZHc7XC1vm2wts8x8ds7Y4Ume7H+N0QtAoSepqSzp69ljEAfeUPxO1cnw7S3vV61rr4PSjsWZ2M1JJJ+kn94p9NqqYuRivrMNsfKqbZEjBSlnJBLMOumbsA9n36HExjLGhOEJJKUlCTmihVRQOgAB8AH5wP/Z".as_bytes().to_vec(),
                             "image/jpeg",
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

    println!(
        "content-type: {:?}, content: {:?}, cluster: {:?}",
        nft.content_type(),
        nft.content(),
        nft.cluster_id()
    );
}

#[test]
fn test_error_data() {
    let data = vec![0, 0, 0, 0, 0];
    let (mut context, tx) = simple_build_context(data, "plain/text", None, 0);

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("Error Data");
}

#[test]
fn test_destroy() {
    let serialized = build_serialized_spore("Hello Spore!", "plain/text");
    let capacity = serialized.total_size() as u64;
    let mut context = Context::default();

    // always success lock
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let spore_type_id = build_script_args(&build_normal_input(&mut context, capacity), 0);
    let spore_input = build_spore_input(
        &mut context,
        &spore_out_point,
        serialized.clone(),
        spore_type_id.clone(),
    );

    let output = build_normal_output(&mut context, capacity);
    let tx = build_simple_tx(
        vec![spore_input],
        vec![output],
        vec![spore_script_dep],
        vec![serialized.as_slice().pack()],
    );

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("try destroy immortal");
}

#[test]
fn test_destroy_immortal() {
    let serialized = build_serialized_spore("Hello Spore!", "plain/text;immortal=true");
    let capacity = serialized.total_size() as u64;
    let mut context = Context::default();

    // always success lock
    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context);
    let spore_type_id = build_script_args(&build_normal_input(&mut context, capacity), 0);
    let spore_input = build_spore_input(
        &mut context,
        &spore_out_point,
        serialized.clone(),
        spore_type_id.clone(),
    );

    let output = build_normal_output(&mut context, capacity);
    let tx = build_simple_tx(
        vec![spore_input],
        vec![output],
        vec![spore_script_dep],
        vec![packed::Bytes::default()],
    );

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("try destroy immortal");
}

#[test]
fn test_destroy_cluster() {
    let cluster = ClusterData::new_builder()
        .name("Spore Cluster".as_bytes().into())
        .description("Test Cluster".as_bytes().into())
        .build();

    let capacity = cluster.total_size() as u64;

    let mut context = Context::default();

    // always success lock
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let cluster_script_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();
    let cluster_type_id = build_script_args(&build_normal_input(&mut context, capacity), 0);
    let type_ = build_spore_type_script(&mut context, &cluster_out_point, cluster_type_id.clone());

    let cluster_input = build_cluster_input(&mut context, cluster, type_.clone());

    let output_cell = build_normal_output(&mut context, capacity);

    let tx = TransactionBuilder::default()
        .input(cluster_input)
        .output(output_cell)
        .output_data(packed::Bytes::default())
        .cell_dep(cluster_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("test destroy cluster");
}

#[test]
fn test_error_type() {
    let error_nft_types = ["plain/;", "text", ";", "-", "plain/", "plain/test;;test=;"];

    for content_type in error_nft_types {
        let (mut context, tx) = simple_build_context(
            "THIS IS A TEST NFT".as_bytes().to_vec(),
            content_type,
            None,
            0,
        );
        let tx = context.complete_tx(tx);

        context.verify_tx(&tx, MAX_CYCLES).expect_err("Error type");
    }
}

#[test]
fn test_extension_1() {
    let mut context = Context::default();

    // always success lock
    let lua_lib_bin: Bytes = Loader::default().load_binary("libckblua.so");
    let lua_lib_out_point = context.deploy_cell(lua_lib_bin.clone());
    let code_hash = CellOutput::calc_data_hash(&lua_lib_bin.clone());
    let lua_lib_dep = CellDep::new_builder()
        .out_point(lua_lib_out_point.clone())
        .build();

    println!("lua lib hash: {}", encode(code_hash.as_slice()));

    let spore_extension_bin: Bytes = Loader::default().load_binary("spore_extension_lua");
    println!(
        "extension hash: {}",
        encode(calc_code_hash(spore_extension_bin.clone()))
    );
    let spore_extension_out_point = context.deploy_cell(spore_extension_bin);
    let spore_extension_script_dep = CellDep::new_builder()
        .out_point(spore_extension_out_point.clone())
        .build();

    let lua_code = String::from("print('hello world')");

    let capacity = lua_code.len() as u64;

    let input_cell = build_normal_input(&mut context, capacity);

    println!(
        "input cell hash: {:?}, out_index: {}",
        input_cell.previous_output().tx_hash().unpack().to_string(),
        0
    );
    let spore_extension_type_id = build_script_args(&input_cell, 0);
    let type_ = build_spore_type_script(
        &mut context,
        &spore_extension_out_point,
        spore_extension_type_id.clone(),
    );

    let spore_out_cell = build_output_cell_with_type_id(&mut context, capacity, type_.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(spore_out_cell)
        .output_data(lua_code.pack())
        .cell_deps(vec![lua_lib_dep, spore_extension_script_dep])
        .build();

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore lua extension");
}
