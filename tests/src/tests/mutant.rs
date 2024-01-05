use ckb_testtool::ckb_types::{core::TransactionBuilder, prelude::*};
use ckb_testtool::context::Context;

use crate::utils::*;
use crate::MAX_CYCLES;

#[test]
fn test_simple_mutant_mint() {
    let mut context = Context::default();

    let (_, lua_lib_dep) = build_spore_materials(&mut context, "libckblua.so");
    let (spore_extension_out_point, spore_extension_script_dep) =
        build_spore_materials(&mut context, "spore_extension_lua");

    let lua_code = String::from("print('hello world')");
    let input_cell = build_normal_input(&mut context);

    println!(
        "input cell hash: {:?}, out_index: {}",
        input_cell.previous_output().tx_hash().unpack().to_string(),
        0
    );
    let spore_extension_type_id = build_type_id(&input_cell, 0);
    let type_ = build_spore_type_script(
        &mut context,
        &spore_extension_out_point,
        spore_extension_type_id.to_vec().into(),
    );

    let spore_out_cell = build_output_cell_with_type_id(&mut context, type_.clone());

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
