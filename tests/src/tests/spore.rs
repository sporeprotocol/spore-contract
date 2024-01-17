use ckb_testtool::ckb_types::{bytes::Bytes, core::TransactionBuilder, packed, prelude::*};
use ckb_testtool::context::Context;

use crate::utils::co_build::*;
use crate::utils::*;
use crate::MAX_CYCLES;

mod simple_spore_mint {
    use super::*;
    use std::fs::File;
    use std::io::{BufReader, Read};

    fn make_simple_spore_mint(output_data: Vec<u8>, content_type: &str) -> Result<u64, String> {
        let mut context = Context::default();
        let tx = build_single_spore_mint_tx(&mut context, output_data, content_type, None, None);
        let tx = context.complete_tx(tx);
        context
            .verify_tx(&tx, MAX_CYCLES)
            .map_err(|err| format!("test simple spore mint: {err}"))
    }

    #[test]
    fn test_simple_spore_mint() {
        make_simple_spore_mint("THIS IS A TEST NFT".as_bytes().to_vec(), "plain/text").unwrap();
    }

    #[test]
    fn test_simple_spore_mint_from_jpeg_image() {
        let jpeg = File::open("resource/test.jpg").unwrap();
        let mut reader = BufReader::new(jpeg);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).expect("Error read to end");

        make_simple_spore_mint(buffer, "image/jpeg").unwrap();
    }

    #[should_panic]
    #[test]
    fn test_simple_spore_mint_failed_with_empty_content() {
        make_simple_spore_mint(vec![], "plain/text").unwrap();
    }

    #[should_panic]
    #[test]
    fn test_simple_spore_mint_failed_with_empty_content_type() {
        make_simple_spore_mint("THIS IS A TEST NFT".as_bytes().to_vec(), "").unwrap();
    }

    #[should_panic = "all failed"]
    #[test]
    fn test_simple_spore_mint_failed_with_wrong_content_types() {
        let output_data = "THIS IS A TEST NFT".as_bytes().to_vec();

        let all_failed = ["plain/;", "text", ";", "-", "plain/", "plain/test;;test=;"]
            .into_iter()
            .map(|content_type| make_simple_spore_mint(output_data.clone(), content_type))
            .all(|v| v.is_err());

        assert!(all_failed == false, "all failed");
    }
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

    let (spore_out_point, spore_script_dep) = build_spore_contract_materials(&mut context, "spore");

    let input_cell_1 = build_normal_input(&mut context);
    let input_cell_2 = build_normal_input(&mut context);
    let input_cell_3 = build_normal_input(&mut context);

    let spore_id_1 = build_type_id(&input_cell_1, 0);
    let spore_type_1 =
        build_spore_type_script(&mut context, &spore_out_point, spore_id_1.to_vec().into());

    let spore_id_2 = build_type_id(&input_cell_1, 2);
    let spore_type_2 =
        build_spore_type_script(&mut context, &spore_out_point, spore_id_2.to_vec().into());

    let spore_out_cell_1 = build_normal_output_cell_with_type(&mut context, spore_type_1.clone());
    let spore_out_cell_2 = build_normal_output_cell_with_type(&mut context, spore_type_2.clone());
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

    let action1 = build_mint_spore_action(&mut context, spore_id_1, serialized.as_slice());
    let action2 = build_mint_spore_action(&mut context, spore_id_2, serialized.as_slice());
    let tx = complete_co_build_message_with_actions(
        tx,
        &[(spore_type_1, action1), (spore_type_2, action2)],
    );

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test multi spore mint");
}

mod spore_multipart_mint {
    use super::*;

    fn make_spore_multipart_mint(output_data: &str, content_type: &str) {
        let mut context = Context::default();
        let tx = build_single_spore_mint_tx(
            &mut context,
            output_data.as_bytes().to_vec(),
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
    fn test_spore_multipart_mint() {
        let output_data = "THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include";
        let content_type = "multipart/mixed;boundary=SporeDefaultBoundary";
        make_spore_multipart_mint(output_data, content_type);
    }

    #[should_panic]
    #[test]
    fn test_spore_multipart_mint_with_wrong_boundary_name() {
        let output_data = "THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include";
        let content_type = "multipart/mixed;boundary=SporeBoundary";
        make_spore_multipart_mint(output_data, content_type);
    }

    #[should_panic]
    #[test]
    fn test_spore_multipart_mint_failed_with_wrong_boundary_type() {
        let output_data = "THIS IS A TEST MULTIPART NFT\n\n--SporeDefaultBoundary\nThis is an extra message I want to include";
        let content_type = "multipart/mixed";
        make_spore_multipart_mint(output_data, content_type)
    }

    #[should_panic]
    #[test]
    fn test_spore_multipart_mint_failed_with_wrong_boundary_data() {
        let output_data =
            "THIS IS A TEST MULTIPART NFT\n\nThis is an extra message I want to include";
        let content_type = "multipart/mixed;boundary=SporeDefaultBoundary;";
        make_spore_multipart_mint(output_data, content_type);
    }
}

mod simple_spore_transfer {
    use super::*;

    fn make_simple_spore_transfer(new_content: Vec<u8>, new_out_index: usize) {
        let mut context = Context::default();
        let (spore_out_point, spore_script_dep) =
            build_spore_contract_materials(&mut context, "spore");
        let normal_input = &build_normal_input(&mut context);

        // build spore cell in Input
        let old_spore_id = build_type_id(&normal_input, 0);
        let old_serialized =
            build_serialized_spore_data("Hello Spore!".as_bytes().to_vec(), "plain/text", None);
        let old_spore_type =
            build_spore_type_script(&mut context, &spore_out_point, old_spore_id.to_vec().into());
        let spore_input = build_spore_input(&mut context, old_spore_type.clone(), old_serialized);

        // build spore cell in Output
        let new_spore_id = build_type_id(&normal_input, new_out_index);
        let new_serialized = build_serialized_spore_data(new_content, "plain/text", None);
        let new_spore_type =
            build_spore_type_script(&mut context, &spore_out_point, new_spore_id.to_vec().into());
        let spore_output = build_normal_output_cell_with_type(&mut context, new_spore_type.clone());

        // build spore transfer tx
        let tx = TransactionBuilder::default()
            .input(spore_input)
            .output(spore_output)
            .output_data(new_serialized.as_slice().pack())
            .cell_dep(spore_script_dep)
            .build();

        let action = build_transfer_spore_action(&mut context, old_spore_id);
        let tx = complete_co_build_message_with_actions(tx, &[(new_spore_type, action)]);

        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test simple spore transfer");
    }

    #[test]
    fn test_simple_spore_transfer() {
        make_simple_spore_transfer("Hello Spore!".as_bytes().to_vec(), 0);
    }

    #[should_panic]
    #[test]
    fn test_simple_spore_transfer_failed_with_wrong_content() {
        make_simple_spore_transfer("Hello New Spore!".as_bytes().to_vec(), 0);
    }

    #[should_panic]
    #[test]
    fn test_simple_spore_transfer_failed_with_wrong_out_index() {
        make_simple_spore_transfer("Hello Spore!".as_bytes().to_vec(), 1);
    }
}

mod spore_mint_from_cluster_lock_proxy {
    use super::*;

    fn make_spore_mint_from_cluster_lock_proxy(append_cluster_dep: bool, lock_args: &[u8]) {
        let mut context = Context::default();
        let (cluster_out_point, _) = build_spore_contract_materials(&mut context, "cluster");

        // build cluster celldep
        let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
        let (cluster_id, _, _, _, cluster_dep) =
            build_cluster_materials(&mut context, &cluster_out_point, cluster, 0, lock_args);

        // build spore mint from cluster tx
        let mut tx = build_single_spore_mint_tx(
            &mut context,
            "Hello Spore!".as_bytes().to_vec(),
            "plain/text",
            None,
            Some(cluster_id),
        );
        if append_cluster_dep {
            tx = tx.as_advanced_builder().cell_dep(cluster_dep).build();
        }
        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test spore mint from lock proxy");
    }

    #[test]
    fn test_spore_mint_from_cluster_lock_proxy() {
        make_spore_mint_from_cluster_lock_proxy(true, &[]);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_cluster_lock_proxy_failed_without_cluster() {
        make_spore_mint_from_cluster_lock_proxy(false, &[]);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_cluster_lock_proxy_failed_with_wrong_cluster() {
        make_spore_mint_from_cluster_lock_proxy(true, &[1]);
    }
}

mod spore_mint_from_cluster_transfer {
    use super::*;

    fn make_spore_mint_from_cluster_transfer(add_cluster_action: bool, add_cluster_dep: bool) {
        let mut context = Context::default();
        let (cluster_out_point, cluster_contract_dep) =
            build_spore_contract_materials(&mut context, "cluster");

        // build cluster materials
        let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
        let (cluster_id, cluster_type, cluster_input, cluster_output, cluster_dep) =
            build_cluster_materials(&mut context, &cluster_out_point, cluster.clone(), 0, &[]);

        // build spore mint from cluster transfer tx
        let action = co_build::build_transfer_cluster_action(&mut context, cluster_id);
        let cluster_action = if add_cluster_action {
            vec![(cluster_type, action)]
        } else {
            vec![]
        };
        let mut tx = build_single_spore_mint_tx_with_extra_action(
            &mut context,
            "Hello Spore!".as_bytes().to_vec(),
            "plain/text",
            None,
            Some(cluster_id),
            cluster_action,
        );
        tx = tx
            .as_advanced_builder()
            .input(cluster_input)
            .output(cluster_output)
            .output_data(cluster.as_bytes().pack())
            .cell_dep(cluster_contract_dep)
            .build();
        if add_cluster_dep {
            tx = tx.as_advanced_builder().cell_dep(cluster_dep).build();
        }
        tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test spore mint from cluster transfer");
    }

    #[test]
    fn test_spore_mint_from_cluster_transfer() {
        make_spore_mint_from_cluster_transfer(true, true);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_cluster_transfer_failed_with_no_cluster_action() {
        make_spore_mint_from_cluster_transfer(false, true);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_cluster_transfer_failed_with_no_cluster_dep() {
        make_spore_mint_from_cluster_transfer(true, false);
    }
}

mod spore_mint_from_agent_lock_proxy {
    use super::*;
    use ckb_testtool::ckb_hash::blake2b_256;

    fn make_spore_mint_from_agent_lock_proxy(add_cluster_dep: bool) {
        let mut context = Context::default();
        let (cluster_out_point, _) = build_spore_contract_materials(&mut context, "cluster");
        let (agent_out_point, _) = build_spore_contract_materials(&mut context, "cluster_agent");

        // build cluster materials
        let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
        let (cluster_id, _, _, _, cluster_dep) =
            build_cluster_materials(&mut context, &cluster_out_point, cluster.clone(), 0, &[]);

        // build cluster agent
        let proxy_type_hash = blake2b_256("12345678");
        let (_, _, _, agent_dep) = build_agent_materials(
            &mut context,
            &agent_out_point,
            &cluster_id,
            &proxy_type_hash,
        );

        // build spore mint from cluster transfer tx
        let mut tx = build_single_spore_mint_tx(
            &mut context,
            "Hello Spore!".as_bytes().to_vec(),
            "plain/text",
            None,
            Some(cluster_id),
        );
        tx = tx.as_advanced_builder().cell_dep(agent_dep).build();
        if add_cluster_dep {
            tx = tx.as_advanced_builder().cell_dep(cluster_dep).build();
        }
        tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test spore mint from cluster transfer");
    }

    #[test]
    fn test_spore_mint_from_agent_lock_proxy() {
        make_spore_mint_from_agent_lock_proxy(true);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_agent_lock_proxy_failed_with_no_cluster_dep() {
        make_spore_mint_from_agent_lock_proxy(false);
    }
}

mod spore_mint_from_agent_transfer {
    use super::*;
    use ckb_testtool::ckb_hash::blake2b_256;

    fn make_spore_mint_from_agent_transfer(add_cluster_dep: bool, add_agent_action: bool) {
        let mut context = Context::default();
        let (cluster_out_point, _) = build_spore_contract_materials(&mut context, "cluster");
        let (agent_out_point, agent_script_dep) =
            build_spore_contract_materials(&mut context, "cluster_agent");

        // build cluster materials
        let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
        let (cluster_id, _, _, _, cluster_dep) =
            build_cluster_materials(&mut context, &cluster_out_point, cluster.clone(), 0, &[]);

        // build cluster agent
        let proxy_type_hash = blake2b_256("12345678");
        let (agent_type, agent_input, agent_output, _) = build_agent_materials(
            &mut context,
            &agent_out_point,
            &cluster_id,
            &proxy_type_hash,
        );

        // build spore mint from cluster transfer tx
        let action = co_build::build_transfer_agent_action(&mut context, cluster_id);
        let agent_action = if add_agent_action {
            vec![(agent_type, action)]
        } else {
            vec![]
        };
        let mut tx = build_single_spore_mint_tx_with_extra_action(
            &mut context,
            "Hello Spore!".as_bytes().to_vec(),
            "plain/text",
            None,
            Some(cluster_id),
            agent_action,
        );
        tx = tx
            .as_advanced_builder()
            .input(agent_input)
            .output(agent_output)
            .output_data(proxy_type_hash.to_vec().pack())
            .cell_dep(agent_script_dep)
            .build();
        if add_cluster_dep {
            tx = tx.as_advanced_builder().cell_dep(cluster_dep).build();
        }
        tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test spore mint from cluster transfer");
    }

    #[test]
    fn test_spore_mint_from_agent_transfer() {
        make_spore_mint_from_agent_transfer(true, true);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_agent_transfer_failed_with_no_cluster_dep() {
        make_spore_mint_from_agent_transfer(false, true);
    }

    #[should_panic]
    #[test]
    fn test_spore_mint_from_agent_transfer_failed_with_no_agent_action() {
        make_spore_mint_from_agent_transfer(true, false);
    }
}

mod simple_spore_destroy {
    use super::*;

    fn make_simple_spore_destroy(content_type: &str) {
        let mut context = Context::default();
        let serialized =
            build_serialized_spore_data("Hello Spore!".as_bytes().to_vec(), content_type, None);
        let (spore_out_point, spore_script_dep) =
            build_spore_contract_materials(&mut context, "spore");

        let normal_input = build_normal_input(&mut context);
        let spore_type_id = build_type_id(&normal_input, 0);
        let type_ = build_spore_type_script(
            &mut context,
            &spore_out_point,
            spore_type_id.to_vec().into(),
        );
        let spore_input = build_spore_input(&mut context, type_.clone(), serialized.clone());

        let normal_output = build_normal_output(&mut context);
        let tx = TransactionBuilder::default()
            .input(spore_input)
            .output(normal_output)
            .output_data(Default::default())
            .cell_dep(spore_script_dep)
            .build();

        let action = build_burn_spore_action(&mut context, spore_type_id);
        let tx = complete_co_build_message_with_actions(tx, &[(type_, action)]);

        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("try destroy spore");
    }

    #[test]
    fn test_simple_spore_destroy() {
        make_simple_spore_destroy("plain/text");
    }

    #[should_panic]
    #[test]
    fn test_simple_spore_destroy_with_immortal() {
        make_simple_spore_destroy("plain/text;immortal=true");
    }
}
