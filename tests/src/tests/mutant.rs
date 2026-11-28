use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{ScriptHashType, TransactionBuilder},
    prelude::*,
};
use ckb_testtool::context::Context;

use crate::utils::*;
use crate::MAX_CYCLES;

#[test]
fn cluster_mint_resolves_the_declared_lua_mutant() {
    let mut context = Context::default();
    let (cluster_code, cluster_dep) = build_spore_contract_materials(&mut context, "cluster");
    let (lua_code, _) = build_spore_contract_materials(&mut context, "spore_extension_lua");
    let input = build_normal_input(&mut context);
    let cluster_id = build_type_id(&input, 0);
    let mutant_id = [7; 32];
    assert_ne!(mutant_id, cluster_id);
    let cluster = build_spore_type_script(&mut context, &cluster_code, cluster_id.to_vec().into());
    let output = build_normal_output_cell_with_type(&mut context, cluster);
    let data = build_serialized_cluster_data("lua cluster", "declared rule")
        .as_builder()
        .mutant_id(mutant_id.as_slice().into())
        .build();
    let base = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(data.as_slice().pack())
        .cell_dep(cluster_dep)
        .build();
    let valid = build_spore_type_script_with_payment(&mut context, &lua_code, &mutant_id, 1);
    let dep = build_normal_cell_dep(&mut context, &[], valid);
    let tx = context.complete_tx(base.as_advanced_builder().cell_dep(dep).build());
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("type-bound Lua mutant with a fee");
    let wrong = build_spore_type_script(&mut context, &lua_code, cluster_id.to_vec().into());
    let dep = build_normal_cell_dep(&mut context, &[], wrong);
    let tx = context.complete_tx(base.as_advanced_builder().cell_dep(dep).build());
    let error = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert!(format!("{error:?}").contains("error code 54"));

    if std::env::var("SPORE_TEST_CLUSTER_NETWORK").as_deref() != Ok("mainnet") {
        let binary = include_bytes!("../../fixtures/published-testnet/spore_extension_lua.bin");
        let legacy_code = context.deploy_cell(Bytes::from_static(binary));
        let legacy = context.build_script_with_hash_type(
            &legacy_code,
            ScriptHashType::Data1,
            mutant_id.to_vec().into(),
        );
        let dep = build_normal_cell_dep(&mut context, &[], legacy);
        let tx = context.complete_tx(base.as_advanced_builder().cell_dep(dep).build());
        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("published historical Lua mutant");
    }
}

#[test]
fn test_simple_mutant_mint() {
    let mut context = Context::default();

    let (_, lua_lib_dep) = build_spore_contract_materials(&mut context, "libckblua.so");
    let (spore_extension_out_point, spore_extension_script_dep) =
        build_spore_contract_materials(&mut context, "spore_extension_lua");

    let lua_code = String::from("print('hello world')");
    let input_cell = build_normal_input(&mut context);

    println!(
        "input cell hash: {:?}, out_index: {}",
        input_cell.previous_output().tx_hash().unpack().to_string(),
        0
    );
    let mutant_id = build_type_id(&input_cell, 0);
    let type_ = build_spore_type_script(
        &mut context,
        &spore_extension_out_point,
        mutant_id.to_vec().into(),
    );

    let mutant_cell_output = build_normal_output_cell_with_type(&mut context, type_.clone());

    let tx = TransactionBuilder::default()
        .input(input_cell)
        .output(mutant_cell_output)
        .output_data(lua_code.pack())
        .cell_deps(vec![lua_lib_dep, spore_extension_script_dep])
        .build();

    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test mint mutant_cell");
}

#[test]
fn test_simple_mutant_spore_mint_without_cluster() {
    let mut context = Context::default();

    let lua_code = "print('hello world')";
    let (tx, mutant_id) = build_single_mutant_celldep_tx(&mut context, lua_code, 1);

    println!("mutant_id: {mutant_id:?}");
    let content_type = format!("plain/test;mutant[]={}", hex::encode(mutant_id));
    let (output_data, normal_input, spore_output, spore_celldep) = build_spore_output_materials(
        &mut context,
        "mutant spore".as_bytes().to_vec(),
        &content_type,
        0,
        None,
    );

    let tx = tx
        .as_advanced_builder()
        .input(normal_input)
        .output(spore_output)
        .output_data(output_data.as_bytes().pack())
        .cell_dep(spore_celldep)
        .build();
    let tx = context.complete_tx(tx);

    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("test mint mutant spore cell (no cluster)");
}
