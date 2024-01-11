use ckb_testtool::ckb_types::H256;
use ckb_testtool::ckb_types::{core::TransactionBuilder, packed, prelude::*};
use ckb_testtool::context::Context;

use crate::utils::co_build::*;
use crate::utils::*;
use crate::MAX_CYCLES;

#[test]
fn test_simple_cluster_mint() {
    let mut context = Context::default();

    let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
    let (cluster_out_point, cluster_script_dep) = build_spore_materials(&mut context, "cluster");
    let input_cell = build_normal_input(&mut context);
    let cluster_type_id = build_type_id(&input_cell, 0);
    let type_ = build_spore_type_script(
        &mut context,
        &cluster_out_point,
        cluster_type_id.to_vec().into(),
    );
    let cluster_out_cell = build_output_cell_with_type_id(&mut context, type_.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(cluster_out_cell)
        .output_data(cluster.as_slice().pack())
        .cell_dep(cluster_script_dep)
        .build();

    let action = build_cluster_create_action(&mut context, cluster_type_id, cluster.as_slice());
    let tx = complete_co_build_message_with_actions(tx, &[(type_, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test simple spore mint");
}

#[test]
fn test_simple_spore_mint_with_cluster() {
    let cluster_id = H256::from_trimmed_str("12345678".clone())
        .expect("parse cluster id")
        .0;
    let serialized = build_serialized_spore_data(
        "Hello Spore!".as_bytes().to_vec(),
        "plain/text",
        Some(cluster_id.to_vec()),
    );

    let mut context = Context::default();
    let tx = build_single_spore_mint_in_cluster_tx(&mut context, serialized, cluster_id);
    let tx = context.complete_tx(tx);

    // run
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
}

#[test]
fn test_cluster_agent_mint() {
    let mut context = Context::default();

    let input_cell = build_normal_input(&mut context);

    // cluster
    let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
    let (cluster_out_point, cluster_script_dep) = build_spore_materials(&mut context, "cluster");
    let cluster_id = build_type_id(&input_cell, 0);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_id.to_vec().into());
    let cluster_dep = build_normal_cell_dep(&mut context, cluster.as_slice(), cluster_type);

    // proxy
    let (proxy_out_point, proxy_script_dep) = build_spore_materials(&mut context, "cluster_proxy");
    let proxy_id = build_type_id(&input_cell, 1);
    let proxy_type_arg = vec![proxy_id.to_vec(), vec![1]].concat();
    let proxy_type = build_spore_type_script(
        &mut context,
        &proxy_out_point,
        proxy_type_arg.clone().into(),
    );
    let proxy_dep = build_normal_cell_dep(&mut context, &cluster_id, proxy_type.clone());
    let proxy_type_hash = proxy_type.unwrap_or_default().calc_script_hash();

    // agent
    let (agent_out_point, agent_script_dep) = build_spore_materials(&mut context, "cluster_agent");
    let agent_type =
        build_spore_type_script(&mut context, &agent_out_point, cluster_id.to_vec().into());
    let agent_out_cell = build_output_cell_with_type_id(&mut context, agent_type.clone())
        .as_builder()
        .capacity((UNIFORM_CAPACITY + 10).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(agent_out_cell)
        .output_data(proxy_type_hash.as_slice().pack())
        .cell_deps(vec![
            cluster_script_dep,
            proxy_script_dep,
            agent_script_dep,
            cluster_dep,
            proxy_dep,
        ])
        .build();

    let action = build_agent_create_action(&mut context, cluster_id);
    let tx = complete_co_build_message_with_actions(tx, &[(agent_type, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test cluster_agent create");
}

#[test]
fn test_cluster_proxy_mint() {
    let mut context = Context::default();

    let input_cell = build_normal_input(&mut context);

    // cluster
    let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
    let (cluster_out_point, cluster_script_dep) = build_spore_materials(&mut context, "cluster");
    let cluster_id = build_type_id(&input_cell, 1);
    let cluster_type =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_id.to_vec().into());
    let cluster_dep = build_normal_cell_dep(&mut context, cluster.as_slice(), cluster_type);

    // proxy
    let (proxy_out_point, proxy_script_dep) = build_spore_materials(&mut context, "cluster_proxy");
    let proxy_id = build_type_id(&input_cell, 0);
    let proxy_type =
        build_spore_type_script(&mut context, &proxy_out_point, proxy_id.to_vec().into());
    let proxy_out_cell = build_output_cell_with_type_id(&mut context, proxy_type.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(proxy_out_cell)
        .output_data(cluster_id.to_vec().pack())
        .cell_deps(vec![cluster_script_dep, proxy_script_dep, cluster_dep])
        .build();

    let action = build_proxy_create_action(&mut context, cluster_id, proxy_id);
    let tx = complete_co_build_message_with_actions(tx, &[(proxy_type, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore mint with lock proxy");
}

#[should_panic]
#[test]
fn test_simple_cluster_destroy_failed() {
    let mut context = Context::default();

    let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");

    let (cluster_out_point, cluster_script_dep) = build_spore_materials(&mut context, "cluster");
    let cluster_id = build_type_id(&build_normal_input(&mut context), 0);
    let type_ =
        build_spore_type_script(&mut context, &cluster_out_point, cluster_id.to_vec().into());

    let cluster_input = build_cluster_input(&mut context, cluster, type_.clone());
    let output_cell = build_normal_output(&mut context);

    let tx = TransactionBuilder::default()
        .input(cluster_input)
        .output(output_cell)
        .output_data(packed::Bytes::default())
        .cell_dep(cluster_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test destroy cluster");
}
