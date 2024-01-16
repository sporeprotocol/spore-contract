#![allow(dead_code)]

use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_hash::{new_blake2b, Blake2bBuilder};
use ckb_testtool::ckb_types::core::ScriptHashType;
use ckb_testtool::ckb_types::{
    bytes::Bytes, core::TransactionBuilder, core::TransactionView, packed, packed::*, prelude::*,
};
use ckb_testtool::context::Context;
use spore_types::generated::action::SporeActionUnion;
use std::num::ParseIntError;

use spore_types::generated::spore_types::{ClusterData, SporeData};
use spore_types::NativeNFTData;

use crate::Loader;

pub mod co_build;
mod internal;

pub const UNIFORM_CAPACITY: u64 = 1000u64;

pub fn build_serialized_cluster_data(name: &str, description: &str) -> ClusterData {
    ClusterData::new_builder()
        .name(name.as_bytes().into())
        .description(description.as_bytes().into())
        .build()
}

pub fn build_serialized_spore_data(
    nft_content: Vec<u8>,
    nft_type: &str,
    cluster_id: Option<Vec<u8>>,
) -> SporeData {
    let nft = NativeNFTData {
        content: nft_content,
        content_type: nft_type.to_owned(),
        cluster_id,
    };
    SporeData::from(nft)
}

pub fn build_type_id(first_input: &CellInput, out_index: usize) -> [u8; 32] {
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(first_input.as_slice());
    blake2b.update(&(out_index).to_le_bytes());
    let mut verify_id = [0; 32];
    blake2b.finalize(&mut verify_id);
    verify_id
}

pub fn build_spore_type_script(
    context: &mut Context,
    out_point: &OutPoint,
    args: Bytes,
) -> Option<Script> {
    context.build_script_with_hash_type(out_point, ScriptHashType::Data1, args)
}

pub fn build_spore_input(
    context: &mut Context,
    spore_type: Option<Script>,
    spore_data: SporeData,
) -> CellInput {
    let input_ckb = spore_data.total_size() as u64;
    internal::build_input(
        context,
        input_ckb,
        spore_type,
        Bytes::copy_from_slice(spore_data.as_slice()),
        Default::default(),
    )
}

pub fn build_cluster_input(
    context: &mut Context,
    cluster_data: ClusterData,
    type_: Option<Script>,
) -> CellInput {
    let input_ckb = cluster_data.total_size() as u64;
    internal::build_input(
        context,
        input_ckb,
        type_,
        Bytes::copy_from_slice(cluster_data.as_slice()),
        Default::default(),
    )
}

pub fn build_agent_proxy_input(
    context: &mut Context,
    cell_data: &[u8],
    type_: Option<Script>,
) -> CellInput {
    let input_ckb = cell_data.len() as u64;
    internal::build_input(
        context,
        input_ckb,
        type_,
        Bytes::copy_from_slice(cell_data),
        Default::default(),
    )
}

pub fn build_normal_input(context: &mut Context) -> CellInput {
    internal::build_input(
        context,
        UNIFORM_CAPACITY,
        None,
        Bytes::new(),
        Default::default(),
    )
}

pub fn build_normal_output_cell_with_type(
    context: &mut Context,
    type_: Option<Script>,
) -> CellOutput {
    internal::build_output(context, UNIFORM_CAPACITY, type_, Default::default())
}

pub fn build_normal_output(context: &mut Context) -> CellOutput {
    internal::build_output(context, UNIFORM_CAPACITY, None, Default::default())
}

pub fn build_normal_cell_dep(context: &mut Context, data: &[u8], type_: Option<Script>) -> CellDep {
    build_normal_cell_dep_with_lock_args(context, data, type_, Default::default())
}

pub fn build_normal_cell_dep_with_lock_args(
    context: &mut Context,
    data: &[u8],
    type_: Option<Script>,
    lock_args: &[u8],
) -> CellDep {
    let outpoint = internal::build_outpoint(
        context,
        data.len() as u64,
        type_,
        Bytes::copy_from_slice(data),
        lock_args.to_vec().into(),
    );
    CellDep::new_builder().out_point(outpoint).build()
}

pub fn build_spore_contract_materials(
    context: &mut Context,
    binary_name: &str,
) -> (OutPoint, CellDep) {
    let binary = Loader::default().load_binary(binary_name);
    let out_point = context.deploy_cell(binary);
    let script_dep = CellDep::new_builder().out_point(out_point.clone()).build();
    (out_point, script_dep)
}

pub fn build_cluster_materials(
    context: &mut Context,
    cluster_out_point: &OutPoint,
    cluster_data: ClusterData,
    cluster_out_index: usize,
    lock_args: &[u8],
) -> ([u8; 32], Option<Script>, CellInput, CellOutput, CellDep) {
    let normal_input = build_normal_input(context);
    let cluster_id = build_type_id(&normal_input, cluster_out_index);
    let cluster_type =
        build_spore_type_script(context, cluster_out_point, cluster_id.to_vec().into());
    let cluster_input = build_cluster_input(context, cluster_data.clone(), cluster_type.clone());
    let cluster_output = build_normal_output_cell_with_type(context, cluster_type.clone());
    let cluster_dep = build_normal_cell_dep_with_lock_args(
        context,
        cluster_data.as_slice(),
        cluster_type.clone(),
        lock_args,
    );
    (
        cluster_id,
        cluster_type,
        cluster_input,
        cluster_output,
        cluster_dep,
    )
}

pub fn build_agent_materials(
    context: &mut Context,
    agent_out_point: &OutPoint,
    cluster_id: &[u8; 32],
    proxy_type_hash: &[u8; 32],
) -> (Option<Script>, CellInput, CellOutput, CellDep) {
    let agent_type = build_spore_type_script(context, agent_out_point, cluster_id.to_vec().into());
    let agent_input = build_agent_proxy_input(context, proxy_type_hash, agent_type.clone());
    let agent_output = build_normal_output_cell_with_type(context, agent_type.clone());
    let agent_dep = build_normal_cell_dep(context, proxy_type_hash, agent_type.clone());
    (agent_type, agent_input, agent_output, agent_dep)
}

pub fn build_single_spore_mint_tx_with_extra_action(
    context: &mut Context,
    output_data: Vec<u8>,
    content_type: &str,
    input_data: Option<SporeData>,
    cluster_id: Option<[u8; 32]>,
    mut actions: Vec<(Option<Script>, SporeActionUnion)>,
) -> TransactionView {
    let output_data =
        build_serialized_spore_data(output_data, content_type, cluster_id.map(|v| v.to_vec()));

    let (spore_out_point, spore_script_dep) = build_spore_contract_materials(context, "spore");
    let (input, type_id) = match input_data {
        None => {
            let input = build_normal_input(context);
            let spore_type_id = build_type_id(&input, 0);
            (input, spore_type_id)
        }
        Some(input_data) => {
            let input = build_normal_input(context);
            let spore_type_id = build_type_id(&input, 0);
            let spore_type =
                build_spore_type_script(context, &spore_out_point, spore_type_id.to_vec().into());
            let spore_input = build_spore_input(context, spore_type, input_data);
            (spore_input, spore_type_id)
        }
    };
    let spore_type = build_spore_type_script(context, &spore_out_point, type_id.to_vec().into());
    let spore_output = build_normal_output_cell_with_type(context, spore_type.clone());
    let tx = TransactionBuilder::default()
        .input(input)
        .output(spore_output)
        .output_data(output_data.as_slice().pack())
        .cell_dep(spore_script_dep)
        .build();

    let action = co_build::build_mint_spore_action(context, type_id, output_data.as_slice());
    actions.push((spore_type, action));
    co_build::complete_co_build_message_with_actions(tx, &actions)
}

pub fn build_single_spore_mint_tx(
    context: &mut Context,
    output_data: Vec<u8>,
    content_type: &str,
    input_data: Option<SporeData>,
    cluster_id: Option<[u8; 32]>,
) -> TransactionView {
    build_single_spore_mint_tx_with_extra_action(
        context,
        output_data,
        content_type,
        input_data,
        cluster_id,
        vec![],
    )
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

pub fn calc_code_hash(data: Bytes) -> [u8; 32] {
    let mut blake2b = new_blake2b();
    blake2b.update(data.to_vec().as_slice());
    let mut hash = [0u8; 32];
    blake2b.finalize(&mut hash);
    hash
}

pub fn build_single_spore_mint_in_cluster_tx(
    context: &mut Context,
    nft_data: SporeData,
    cluster_id: [u8; 32],
) -> TransactionView {
    let cluster_data = build_serialized_cluster_data("Spore Cluster!", "Spore Description!");
    let nft_bin: Bytes = Loader::default().load_binary("spore");
    let nft_out_point = context.deploy_cell(nft_bin);
    let cluster_bin: Bytes = Loader::default().load_binary("cluster");
    let cluster_out_point = context.deploy_cell(cluster_bin);
    let input_ckb = nft_data.total_size() as u64;

    let output_ckb = input_ckb;
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // build lock script
    let lock_script = internal::build_always_success_script(context, Default::default());
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
            .build(),
        Bytes::new(),
    );

    let cluster_script = context.build_script_with_hash_type(
        &cluster_out_point,
        ScriptHashType::Data1,
        cluster_id.to_vec().into(),
    );

    let cluster_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((cluster_data.total_size() as u64).pack())
            .lock(lock_script.clone())
            .type_(cluster_script.pack())
            .build(),
        Bytes::copy_from_slice(cluster_data.as_slice()),
    );

    let cluster_dep = CellDep::new_builder()
        .out_point(cluster_out_point.clone())
        .build();

    let cluster_input = CellInput::new_builder()
        .previous_output(cluster_out_point)
        .build();

    let normal_input = CellInput::new_builder()
        .previous_output(
            context.create_cell(
                CellOutput::new_builder()
                    .capacity(1000000u64.pack())
                    .lock(lock_script.clone())
                    .build(),
                Bytes::new(),
            ),
        )
        .build();

    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();

    let nft_id = {
        let mut blake2b = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        blake2b.update(input.as_slice());
        blake2b.update(&1u64.to_le_bytes());
        let mut verify_id = [0; 32];
        blake2b.finalize(&mut verify_id);
        verify_id
    };

    let nft_script = context.build_script_with_hash_type(
        &nft_out_point,
        ScriptHashType::Data1,
        nft_id.to_vec().into(),
    );

    let nft_script_dep = CellDep::new_builder().out_point(nft_out_point).build();

    let output = CellOutput::new_builder()
        .capacity((output_ckb + cluster_data.total_size() as u64).pack())
        .lock(lock_script.clone())
        .type_(nft_script.pack())
        .build();

    let cluster_output = CellOutput::new_builder()
        .capacity(input_ckb.pack())
        .lock(lock_script.clone())
        .type_(cluster_script.pack())
        .build();

    let normal_output = CellOutput::new_builder()
        .capacity(9999u64.pack())
        .lock(lock_script.clone())
        .build();

    let tx = TransactionBuilder::default()
        .inputs(vec![input, normal_input, cluster_input])
        .outputs(vec![normal_output, output, cluster_output])
        .outputs_data(vec![
            packed::Bytes::default(),
            nft_data.as_slice().pack(),
            cluster_data.as_slice().pack(),
        ])
        .cell_deps(vec![
            lock_script_dep,
            cluster_script_dep,
            nft_script_dep,
            cluster_dep,
        ])
        .build();

    let cluster_transfer = co_build::build_transfer_cluster_action(context, cluster_id);
    let nft_action = co_build::build_mint_spore_action(context, nft_id, nft_data.as_slice());
    co_build::complete_co_build_message_with_actions(
        tx,
        &[(cluster_script, cluster_transfer), (nft_script, nft_action)],
    )
}
