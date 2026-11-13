//! Code identity tests. Code-Cell spending authorization belongs to its Lock.
use crate::{utils::*, MAX_CYCLES};
use ckb_testtool::{
    ckb_types::{bytes::Bytes, core::TransactionView, packed::*, prelude::*},
    context::Context,
};

fn code_type(role: &str) -> Script {
    let mut code = [0u8; 32];
    code[25..].copy_from_slice(b"TYPE_ID");
    Script::new_builder()
        .code_hash(code.pack())
        .hash_type(1u8.into())
        .args(code_type_id_args(role).pack())
        .build()
}

fn replace_code(
    context: &mut Context,
    tx: &TransactionView,
    change_identity: bool,
) -> TransactionView {
    let expected = code_type("spore");
    let mut deps: Vec<CellDep> = tx.cell_deps().into_iter().collect();
    let index = deps
        .iter()
        .position(|dep| {
            context
                .get_cell(&dep.out_point())
                .unwrap()
                .0
                .type_()
                .to_opt()
                == Some(expected.clone())
        })
        .expect("Spore code dependency");
    let previous = deps[index].out_point();
    let (mut output, data) = context.get_cell(&previous).unwrap();
    let mut bytes = data.to_vec();
    if change_identity {
        let other = expected.as_builder().args(vec![0xff; 32].pack()).build();
        output = output.as_builder().type_(Some(other).pack()).build();
    } else {
        // Equivalent ELF with different content hash, modelling a reviewed successor.
        bytes.push(0);
        assert_ne!(
            CellOutput::calc_data_hash(&data),
            CellOutput::calc_data_hash(&Bytes::from(bytes.clone()))
        );
    }
    let successor = context.create_cell(output, bytes.into());
    context.cells.remove(&previous); // the previous code Cell is no longer live
    deps[index] = deps[index]
        .clone()
        .as_builder()
        .out_point(successor)
        .build();
    tx.as_advanced_builder().set_cell_deps(deps).build()
}

#[test]
fn type_bound_code_survives_a_changed_binary_and_outpoint() {
    let mut context = Context::default();
    let tx = build_single_spore_mint_tx(&mut context, vec![1], "text/plain", None, None);
    let tx = context.complete_tx(tx);
    context.verify_tx(&tx, MAX_CYCLES).expect("original code");
    let upgraded = replace_code(&mut context, &tx, false);
    assert_eq!(
        tx.outputs().get(0).unwrap().type_(),
        upgraded.outputs().get(0).unwrap().type_()
    );
    context
        .verify_tx(&upgraded, MAX_CYCLES)
        .expect("same code Type ID resolves the successor");
}

#[test]
fn identical_binary_at_another_type_id_cannot_substitute_for_code() {
    let mut context = Context::default();
    let tx = build_single_spore_mint_tx(&mut context, vec![1], "text/plain", None, None);
    let tx = context.complete_tx(tx);
    context.verify_tx(&tx, MAX_CYCLES).expect("original code");
    let substituted = replace_code(&mut context, &tx, true);
    assert!(context.verify_tx(&substituted, MAX_CYCLES).is_err());
}
