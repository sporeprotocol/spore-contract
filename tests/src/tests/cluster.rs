use ckb_testtool::ckb_hash::blake2b_256;
use ckb_testtool::ckb_types::{core::TransactionBuilder, packed, prelude::*};
use ckb_testtool::context::Context;

use crate::utils::co_build::*;
use crate::utils::*;
use crate::MAX_CYCLES;

mod simple_cluster_mint {
    use super::*;

    fn make_simple_cluster_mint(cluster_out_index: usize) {
        let mut context = Context::default();
        let input_cell = build_normal_input(&mut context);

        let cluster = build_serialized_cluster_data("Spore Cluster", "Test Cluster");
        let (cluster_out_point, cluster_script_dep) =
            build_spore_materials(&mut context, "cluster");
        let cluster_type_id = build_type_id(&input_cell, cluster_out_index);
        let type_ = build_spore_type_script(
            &mut context,
            &cluster_out_point,
            cluster_type_id.to_vec().into(),
        );
        let cluster_out_cell = build_normal_output_cell_with_type(&mut context, type_.clone());

        let tx = TransactionBuilder::default()
            .input(input_cell)
            .output(cluster_out_cell)
            .output_data(cluster.as_slice().pack())
            .cell_dep(cluster_script_dep)
            .build();

        let action = build_mint_cluster_action(&mut context, cluster_type_id, cluster.as_slice());
        let tx = complete_co_build_message_with_actions(tx, &[(type_, action)]);

        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test simple spore mint");
    }

    #[test]
    fn test_simple_cluster_mint() {
        make_simple_cluster_mint(0);
    }

    #[should_panic]
    #[test]
    fn test_simple_cluster_mint_failed_with_wrong_out_index() {
        make_simple_cluster_mint(1);
    }
}

#[cfg(test)]
mod simple_cluster_transfer {
    use super::*;

    fn make_simple_cluster_transfer(new_cluster_data_desc: &str, new_cluster_out_index: usize) {
        let mut context = Context::default();
        let normal_cell = build_normal_input(&mut context);
        let (cluster_out_point, cluster_script_dep) =
            build_spore_materials(&mut context, "cluster");

        // cluster in Input
        let old_cluster_data =
            build_serialized_cluster_data("Spore Cluster", "Test Cluster Transfer");
        let old_cluster_type_id = build_type_id(&normal_cell, 0);
        let type_ = build_spore_type_script(
            &mut context,
            &cluster_out_point,
            old_cluster_type_id.to_vec().into(),
        );
        let old_cluster_cell = build_cluster_input(&mut context, old_cluster_data, type_);

        // cluster in Output
        let new_cluster_data =
            build_serialized_cluster_data("Spore Cluster", new_cluster_data_desc);
        let new_cluster_type_id = build_type_id(&normal_cell, new_cluster_out_index);
        let type_ = build_spore_type_script(
            &mut context,
            &cluster_out_point,
            new_cluster_type_id.to_vec().into(),
        );
        let new_cluster_cell = build_normal_output_cell_with_type(&mut context, type_.clone());

        // build cluster transfer tx
        let tx = TransactionBuilder::default()
            .input(old_cluster_cell)
            .output(new_cluster_cell)
            .output_data(new_cluster_data.as_slice().pack())
            .cell_dep(cluster_script_dep)
            .build();

        let action = build_transfer_cluster_action(&mut context, new_cluster_type_id);
        let tx = complete_co_build_message_with_actions(tx, &[(type_, action)]);

        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test cluster transfer");
    }

    #[test]
    fn test_simple_cluster_transfer() {
        make_simple_cluster_transfer("Test Cluster Transfer", 0);
    }

    #[should_panic]
    #[test]
    fn test_simple_cluster_transfer_failed_with_wrong_cluster_data() {
        make_simple_cluster_transfer("Test New Cluster Transfer", 0);
    }

    #[should_panic]
    #[test]
    fn test_simple_cluster_transfer_failed_with_wrong_type_id() {
        make_simple_cluster_transfer("Test Cluster Transfer", 1);
    }
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

#[test]
fn test_simple_spore_mint_with_cluster() {
    let cluster_id = blake2b_256("12345678");
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
    let agent_out_cell = build_normal_output_cell_with_type(&mut context, agent_type.clone())
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

    let action = build_mint_agent_action(&mut context, cluster_id, proxy_id);
    let tx = complete_co_build_message_with_actions(tx, &[(agent_type, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test cluster_agent create");
}

mod cluster_agent_transfer {
    use super::*;

    fn make_cluster_agent_transfer(new_agent_data: &[u8], new_cluster_out_index: usize) {
        let mut context = Context::default();
        let input_cell = build_normal_input(&mut context);
        let (agent_out_point, agent_script_dep) =
            build_spore_materials(&mut context, "cluster_agent");

        // agent in Input
        let old_cluster_id = build_type_id(&input_cell, 0);
        let old_agent_data = blake2b_256("12345676890");
        let old_agent_type = build_spore_type_script(
            &mut context,
            &agent_out_point,
            old_cluster_id.to_vec().into(),
        );
        let old_agent_cell = build_agent_proxy_input(&mut context, &old_agent_data, old_agent_type);

        // agent in Output
        let new_cluster_id = build_type_id(&input_cell, new_cluster_out_index);
        let new_agent_type = build_spore_type_script(
            &mut context,
            &agent_out_point,
            new_cluster_id.to_vec().into(),
        );
        let new_agent_cell =
            build_normal_output_cell_with_type(&mut context, new_agent_type.clone());

        // build agent transfer tx
        let tx = TransactionBuilder::default()
            .input(old_agent_cell)
            .output(new_agent_cell)
            .output_data(new_agent_data.pack())
            .cell_dep(agent_script_dep)
            .build();

        let action = build_transfer_agent_action(&mut context, new_cluster_id);
        let tx = complete_co_build_message_with_actions(tx, &[(new_agent_type, action)]);

        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test agent transfer");
    }

    #[test]
    fn test_cluster_agent_transfer() {
        let proxy_type_hash = blake2b_256("12345676890");
        make_cluster_agent_transfer(&proxy_type_hash, 0);
    }

    #[should_panic]
    #[test]
    fn test_cluster_agent_transfer_failed_with_wrong_data() {
        make_cluster_agent_transfer(&[1u8], 0);
    }

    #[should_panic]
    #[test]
    fn test_cluster_agent_transfer_failed_with_wrong_cluster_id() {
        let proxy_type_hash = blake2b_256("12345676890");
        make_cluster_agent_transfer(&proxy_type_hash, 1);
    }
}

#[test]
fn test_cluster_agent_burn() {
    let mut context = Context::default();
    let input_cell = build_normal_input(&mut context);
    let (agent_out_point, agent_script_dep) = build_spore_materials(&mut context, "cluster_agent");

    // agent in Input
    let cluster_id = build_type_id(&input_cell, 0);
    let agent_data = blake2b_256("12345676890");
    let agent_type =
        build_spore_type_script(&mut context, &agent_out_point, cluster_id.to_vec().into());
    let agent_cell = build_agent_proxy_input(&mut context, &agent_data, agent_type.clone());

    // build agent burn tx
    let normal_cell = build_normal_output(&mut context);
    let tx = TransactionBuilder::default()
        .input(agent_cell)
        .output(normal_cell)
        .output_data(Default::default())
        .cell_dep(agent_script_dep)
        .build();

    let action = build_burn_agent_action(&mut context, cluster_id);
    let tx = complete_co_build_message_with_actions(tx, &[(agent_type, action)]);

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test agent burn");
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
    let proxy_out_cell = build_normal_output_cell_with_type(&mut context, proxy_type.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(proxy_out_cell)
        .output_data(cluster_id.to_vec().pack())
        .cell_deps(vec![cluster_script_dep, proxy_script_dep, cluster_dep])
        .build();

    let action = build_mint_proxy_action(&mut context, cluster_id, proxy_id);
    let tx = complete_co_build_message_with_actions(tx, &[(proxy_type, action)]);

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test spore mint with lock proxy");
}

mod cluster_proxy_transfer {
    use super::*;

    fn make_cluster_proxy_transfer(new_proxy_out_index: usize, new_cluster_id: [u8; 32]) {
        let mut context = Context::default();
        let input_cell = build_normal_input(&mut context);
        let (proxy_out_point, proxy_script_dep) =
            build_spore_materials(&mut context, "cluster_proxy");

        // proxy in Input
        let old_cluster_id = blake2b_256("12345678");
        let old_proxy_id = build_type_id(&input_cell, 0);
        let old_proxy_type =
            build_spore_type_script(&mut context, &proxy_out_point, old_proxy_id.to_vec().into());
        let old_proxy_cell = build_agent_proxy_input(&mut context, &old_cluster_id, old_proxy_type);

        // proxy in Output
        let new_proxy_id = build_type_id(&input_cell, new_proxy_out_index);
        let new_proxy_type =
            build_spore_type_script(&mut context, &proxy_out_point, new_proxy_id.to_vec().into());
        let new_proxy_cell =
            build_normal_output_cell_with_type(&mut context, new_proxy_type.clone());

        // build proxy transfer tx
        let tx = TransactionBuilder::default()
            .input(old_proxy_cell)
            .output(new_proxy_cell)
            .output_data(new_cluster_id.to_vec().pack())
            .cell_dep(proxy_script_dep)
            .build();

        let action = build_transfer_proxy_action(&mut context, old_cluster_id, old_proxy_id);
        let tx = complete_co_build_message_with_actions(tx, &[(new_proxy_type, action)]);

        let tx = context.complete_tx(tx);

        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("test proxy transfer");
    }

    #[test]
    fn test_cluster_proxy_transfer() {
        let new_cluster_id = blake2b_256("12345678");
        make_cluster_proxy_transfer(0, new_cluster_id);
    }

    #[should_panic]
    #[test]
    fn test_cluster_proxy_transfer_failed_with_wrong_proxy_id() {
        let new_cluster_id = blake2b_256("12345678");
        make_cluster_proxy_transfer(1, new_cluster_id);
    }

    #[should_panic]
    #[test]
    fn test_cluster_proxy_transfer_failed_with_wrong_cluster_id() {
        let new_cluster_id = blake2b_256("87654321");
        make_cluster_proxy_transfer(0, new_cluster_id);
    }
}

#[test]
fn test_cluster_proxy_burn() {
    let mut context = Context::default();
    let input_cell = build_normal_input(&mut context);
    let (proxy_out_point, proxy_script_dep) = build_spore_materials(&mut context, "cluster_proxy");

    // proxy in Input
    let cluster_id = blake2b_256("12345678");
    let proxy_id = build_type_id(&input_cell, 0);
    let proxy_type =
        build_spore_type_script(&mut context, &proxy_out_point, proxy_id.to_vec().into());
    let proxy_cell = build_agent_proxy_input(&mut context, &cluster_id, proxy_type.clone());

    // build proxy burn tx
    let normal_cell = build_normal_output(&mut context);
    let tx = TransactionBuilder::default()
        .input(proxy_cell)
        .output(normal_cell)
        .output_data(Default::default())
        .cell_dep(proxy_script_dep)
        .build();

    let action = build_burn_proxy_action(&mut context, cluster_id, proxy_id);
    let tx = complete_co_build_message_with_actions(tx, &[(proxy_type, action)]);

    let tx = context.complete_tx(tx);

    context.verify_tx(&tx, MAX_CYCLES).expect("test proxy burn");
}
