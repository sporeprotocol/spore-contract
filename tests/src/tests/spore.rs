use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed, prelude::*};
use ckb_testtool::context::Context;

use crate::utils::co_build::*;
use crate::utils::*;
use crate::MAX_CYCLES;

#[test]
fn test_simple_spore_mint() {
    let mut context = Context::default();
    let tx = build_single_spore_mint_tx(
        &mut context,
        "THIS IS A TEST NFT".as_bytes().to_vec(),
        "plain/text",
        None,
        None,
    );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint");
}

#[test]
fn test_simple_spore_mint_with_extra_cells() {
    let mut context = Context::default();

    let tx = build_single_spore_mint_tx(
        &mut context,
        "THIS IS A TEST NFT".as_bytes().to_vec(),
        "plain/text",
        None,
        None,
    );

    let extra_input_cell_1 = build_normal_input(&mut context);
    let extra_input_cell_2 = build_normal_input(&mut context);
    let extra_output_cell = build_normal_output(&mut context);

    let tx = tx
        .as_advanced_builder()
        .inputs(vec![extra_input_cell_1, extra_input_cell_2])
        .output(extra_output_cell)
        .output_data(Bytes::default().pack())
        .build();

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint with multi normal cells");
}

#[test]
fn test_multi_spores_mint() {
    let mut context = Context::default();

    // multiple mint tx test
    let serialized =
        build_serialized_spore_data("Hello Spore!".as_bytes().to_vec(), "plain/text", None);

    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context, "spore");

    let input_cell_1 = build_normal_input(&mut context);
    let input_cell_2 = build_normal_input(&mut context);
    let input_cell_3 = build_normal_input(&mut context);

    let spore_id_1 = build_type_id(&input_cell_1, 0);
    let spore_type_1 =
        build_spore_type_script(&mut context, &spore_out_point, spore_id_1.to_vec().into());

    let spore_id_2 = build_type_id(&input_cell_1, 2);
    let spore_type_2 =
        build_spore_type_script(&mut context, &spore_out_point, spore_id_2.to_vec().into());

    let spore_out_cell_1 = build_output_cell_with_type_id(&mut context, spore_type_1.clone());
    let spore_out_cell_2 = build_output_cell_with_type_id(&mut context, spore_type_2.clone());
    let output_cell = build_normal_output(&mut context);

    let tx = TransactionBuilder::default()
        .inputs(vec![input_cell_1, input_cell_2, input_cell_3])
        .outputs(vec![spore_out_cell_1, output_cell, spore_out_cell_2])
        .outputs_data(vec![
            serialized.as_slice().pack(),
            packed::Bytes::default(),
            serialized.as_slice().pack(),
        ])
        .cell_dep(spore_script_dep)
        .build();

    let action1 = build_mint_action(&mut context, spore_id_1, serialized.as_slice());
    let action2 = build_mint_action(&mut context, spore_id_2, serialized.as_slice());
    let tx = complete_co_build_message_with_actions(
        tx,
        &[(spore_type_1, action1), (spore_type_2, action2)],
    );

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test multi spore mint");
}

#[test]
fn test_spore_multipart_mint() {
    let mut context = Context::default();
    let content = "THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include";
    let content_type = "multipart/mixed;boundary=SporeDefaultBoundary";
    let tx = build_single_spore_mint_tx(
        &mut context,
        content.as_bytes().to_vec(),
        content_type,
        None,
        None,
    );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test multipart mint");
}

#[test]
fn test_simple_spore_transfer() {
    let serialized =
        build_serialized_spore_data("Hello Spore!".as_bytes().to_vec(), "plain/text", None);
    let mut context = Context::default();

    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context, "spore");
    let spore_type_id = build_type_id(&build_normal_input(&mut context), 0);
    let spore_type = build_spore_type_script(
        &mut context,
        &spore_out_point,
        spore_type_id.to_vec().into(),
    );
    let spore_input = build_spore_input(&mut context, spore_type.clone(), serialized.clone());

    let spore_output = build_output_cell_with_type_id(&mut context, spore_type.clone());
    let tx = TransactionBuilder::default()
        .input(spore_input)
        .output(spore_output)
        .output_data(serialized.as_slice().pack())
        .cell_dep(spore_script_dep)
        .build();

    let action = build_transfer_action(&mut context, spore_type_id);
    let tx = complete_co_build_message_with_actions(tx, &[(spore_type, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore transfer");
}

#[test]
fn test_spore_mint_with_lock_proxy() {
    let mut context = Context::default();

    let input_cell = build_normal_input(&mut context);

    // cluster
    let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
    let (cluster_out_point, _) = build_spore_materials(&mut context, "cluster");
    let cluster_id = build_type_id(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_id.to_vec().into());
    let cluster_dep = build_normal_cell_dep(&mut context, cluster.as_slice(), cluster_type);

    // spore
    let tx = build_single_spore_mint_tx(
        &mut context,
        "Hello Spore!".as_bytes().to_vec(),
        "plain/text",
        None,
        Some(cluster_id),
    );
    let tx = tx.as_advanced_builder().cell_dep(cluster_dep).build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore mint with lock proxy");
}

#[test]
fn test_simple_spore_destroy() {
    let serialized =
        build_serialized_spore_data("Hello Spore!".as_bytes().to_vec(), "plain/text", None);
    let mut context = Context::default();

    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context, "spore");
    let input = build_normal_input(&mut context);
    let spore_type_id = build_type_id(&input, 0);
    let type_ = build_spore_type_script(
        &mut context,
        &spore_out_point,
        spore_type_id.to_vec().into(),
    );
    let spore_input = build_spore_input(&mut context, type_.clone(), serialized.clone());

    let output = build_normal_output(&mut context);
    let tx = TransactionBuilder::default()
        .input(spore_input)
        .output(output)
        .output_data(serialized.as_slice().pack())
        .cell_dep(spore_script_dep)
        .build();

    let action = build_burn_action(&mut context, spore_type_id);
    let tx = complete_co_build_message_with_actions(tx, &[(type_, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("try destroy immortal");
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

        build_single_spore_mint_tx(&mut Context::default(), buffer, "image/jpeg", None, None);
    } else {
        println!("Error while reading file!");
    }
}

#[test]
fn test_read_base64() {
    let mut context = Context::default();
    let content = "/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAoHBwgHBgoICAgLCgoLDhgQDg0NDh0VFhEYIx8lJCIfIiEmKzcvJik0KSEiMEExNDk7Pj4+JS5ESUM8SDc9Pjv/2wBDAQoLCw4NDhwQEBw7KCIoOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozv/wAARCAAKAAoDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAABAf/xAAfEAADAAICAwEBAAAAAAAAAAABAgMEBQARBiExQRT/xAAUAQEAAAAAAAAAAAAAAAAAAAAA/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAwDAQACEQMRAD8AZHc7XC1vm2wts8x8ds7Y4Ume7H+N0QtAoSepqSzp69ljEAfeUPxO1cnw7S3vV61rr4PSjsWZ2M1JJJ+kn94p9NqqYuRivrMNsfKqbZEjBSlnJBLMOumbsA9n36HExjLGhOEJJKUlCTmihVRQOgAB8AH5wP/Z";
    let tx = build_single_spore_mint_tx(
        &mut context,
        content.as_bytes().to_vec(),
        "image/jpeg",
        None,
        None,
    );
    let view = context.complete_tx(tx);
    context.verify_tx(&view, MAX_CYCLES).expect("Error tx");
}

#[test]
fn test_error_data() {
    let mut context = Context::default();
    let content = vec![0, 0, 0, 0, 0];
    let tx = build_single_spore_mint_tx(&mut context, content, "plain/text", None, None);

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("Error Data");
}

#[should_panic]
#[test]
fn test_spore_mint_failed_with_empty_content() {
    let mut context = Context::default();
    let tx = build_single_spore_mint_tx(&mut context, vec![], "plain/text", None, None);
    let tx = context.complete_tx(tx);

    // run
    context.verify_tx(&tx, MAX_CYCLES).expect("Empty Content");
}

#[should_panic]
#[test]
fn test_spore_multipart_mint_failed_with_mixed() {
    let mut context = Context::default();
    let content = "THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include";
    let content_type = "multipart/mixed;";
    let tx = build_single_spore_mint_tx(
        &mut context,
        content.as_bytes().to_vec(),
        content_type,
        None,
        None,
    );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test multipart failure mixed");
}

#[should_panic]
#[test]
fn test_spore_multipart_mint_failed_with_boundary() {
    let mut context = Context::default();
    let content = "THIS IS A TEST MULTIPART NFT\n\nThis is an extra message I want to include";
    let content_type = "multipart/mixed;boundary=SporeDefaultBoundary;";
    let tx = build_single_spore_mint_tx(
        &mut context,
        content.as_bytes().to_vec(),
        content_type,
        None,
        None,
    );
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test multipart failure boundary");
}

#[should_panic]
#[test]
fn test_spore_mint_with_lock_proxy_failed_without_celldep() {
    let mut context = Context::default();

    let input_cell = build_normal_input(&mut context);
    let cluster_id = build_type_id(&input_cell, 0);

    // spore
    let tx = build_single_spore_mint_tx(
        &mut context,
        "Hello Spore!".as_bytes().to_vec(),
        "plain/text",
        None,
        Some(cluster_id),
    );
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore mint with lock proxy failure case");
}

#[should_panic]
#[test]
fn test_simple_spore_destroy_failed_with_immortal() {
    let serialized = build_serialized_spore_data(
        "Hello Spore!".as_bytes().to_vec(),
        "plain/text;immortal=true",
        None,
    );
    let mut context = Context::default();

    let (spore_out_point, spore_script_dep) = build_spore_materials(&mut context, "spore");
    let input = build_normal_input(&mut context);
    let spore_type_id = build_type_id(&input, 0);
    let spore_type = build_spore_type_script(
        &mut context,
        &spore_out_point,
        spore_type_id.to_vec().into(),
    );
    let spore_input = build_spore_input(&mut context, spore_type.clone(), serialized);

    let output = build_normal_output(&mut context);
    let tx = TransactionBuilder::default()
        .input(spore_input)
        .output(output)
        .output_data(packed::Bytes::default())
        .cell_dep(spore_script_dep)
        .build();

    let action = build_burn_action(&mut context, spore_type_id);
    let tx = complete_co_build_message_with_actions(tx, &[(spore_type, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("try destroy immortal");
}

#[should_panic]
#[test]
fn test_simple_spore_mint_failed_with_error_type() {
    let error_nft_types = ["plain/;", "text", ";", "-", "plain/", "plain/test;;test=;"];

    let mut context = Context::default();
    for content_type in error_nft_types {
        let tx = build_single_spore_mint_tx(
            &mut context,
            "THIS IS A TEST NFT".as_bytes().to_vec(),
            content_type,
            None,
            None,
        );
        let tx = context.complete_tx(tx);

        context.verify_tx(&tx, MAX_CYCLES).expect("Error type");
    }
}
