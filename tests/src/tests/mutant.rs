use ckb_testtool::ckb_types::{core::TransactionBuilder, prelude::*};
use ckb_testtool::context::Context;

use crate::utils::*;
use crate::MAX_CYCLES;

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
