//! Explicit release-artifact benchmark; no Lua code or interpreter is loaded.
use super::*;
const PERF_REPEATS: usize = 5;
const PERF_LIMIT: u64 = 500_000_000;

fn perf_point(tag: u8, ordinal: usize) -> OutPoint {
    let mut hash = [0u8; 32];
    hash[0] = tag;
    hash[24..].copy_from_slice(&(ordinal as u64).to_le_bytes());
    OutPoint::new_builder().tx_hash(hash.pack()).build()
}

fn perf_input(context: &mut Context, tag: u8, ordinal: usize) -> CellInput {
    let output = build_normal_output(context);
    let point = perf_point(tag, ordinal);
    context.create_cell_with_out_point(point.clone(), output, Bytes::new());
    CellInput::new_builder().previous_output(point).build()
}

fn perf_measure(context: &Context, tx: &TransactionView) -> u64 {
    let cycles = context
        .verify_tx(tx, PERF_LIMIT)
        .expect("benchmark transaction");
    for _ in 1..PERF_REPEATS {
        assert_eq!(context.verify_tx(tx, PERF_LIMIT).unwrap(), cycles);
    }
    cycles
}

fn perf_rule(context: &mut Context, ordinal: usize) -> (CellDep, [u8; 32], u64) {
    let contract = "spore_extension_rust";
    let (code, dep) = build_spore_contract_materials(context, contract);
    let input = perf_input(context, 1, ordinal);
    let id = build_type_id(&input, 0);
    let script = context
        .build_script_with_hash_type(&code, ScriptHashType::Data2, id.to_vec().into())
        .unwrap();
    let output = build_normal_output_cell_with_type(context, Some(script));
    let data = binary("mutant_equipment");
    let builder = TransactionBuilder::default()
        .input(input)
        .output(output.clone())
        .output_data(data.pack())
        .cell_dep(dep);
    let tx = context.complete_tx(builder.build());
    let cycles = perf_measure(context, &tx);
    let point = perf_point(2, ordinal);
    context.create_cell_with_out_point(point.clone(), output, data);
    (CellDep::new_builder().out_point(point).build(), id, cycles)
}

fn perf_mint(
    context: &mut Context,
    rules: &[(CellDep, [u8; 32])],
    extra: &[CellDep],
) -> TransactionView {
    let ids = rules
        .iter()
        .map(|(_, id)| hex::encode(id))
        .collect::<Vec<_>>()
        .join(",");
    let mime = if rules.is_empty() {
        "application/x.equipment".to_owned()
    } else {
        format!("application/x.equipment;mutant[]={ids}")
    };
    let input = perf_input(context, 3, 0);
    let id = build_type_id(&input, 0);
    let (code, dep) = build_spore_contract_materials(context, "spore");
    let script = build_spore_type_script(context, &code, id.to_vec().into()).unwrap();
    let data = build_serialized_spore_data(vec![3, 4, 7], &mime, None);
    let output = build_normal_output_cell_with_type(context, Some(script.clone()));
    let tx = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(data.as_slice().pack())
        .cell_dep(dep)
        .cell_deps(rules.iter().map(|(dep, _)| dep.clone()).collect::<Vec<_>>())
        .cell_deps(extra.to_vec())
        .build();
    let action = co_build::build_mint_spore_action(context, id, data.as_slice());
    context.complete_tx(co_build::complete_co_build_message_with_actions(
        tx,
        &[(Some(script), action)],
    ))
}

#[test]
#[ignore = "performance report: run explicitly with release artifacts"]
fn native_mutant_performance_report() {
    println!("PERF_METHOD repeats={PERF_REPEATS} limit={PERF_LIMIT} content=3,4,7 payment=none lock=ALWAYS_SUCCESS");
    for (label, count) in [("baseline", 0), ("rust1", 1), ("rust2", 2), ("rust4", 4)] {
        let mut context = Context::new_with_deterministic_rng();
        let mut rules = Vec::new();
        let extra = Vec::new();
        for ordinal in 0..count {
            let (dep, id, cycles) = perf_rule(&mut context, ordinal);
            if ordinal == 0 {
                println!("PERF creation rust {cycles}");
            }
            rules.push((dep, id));
        }
        let tx = perf_mint(&mut context, &rules, &extra);
        let mint_cycles = perf_measure(&context, &tx);
        let transfer = move_spore(&mut context, &tx, false);
        let burn = move_spore(&mut context, &tx, true);
        let transfer_cycles = perf_measure(&context, &transfer);
        let burn_cycles = perf_measure(&context, &burn);
        println!("PERF {label} mint={mint_cycles} transfer={transfer_cycles} burn={burn_cycles} deps={} tx_bytes={}",
            tx.cell_deps().len(), tx.data().as_slice().len());
        // Measured transactions still enforce the recipe and co-build validation.
        if !rules.is_empty() {
            let invalid = replace_content(&mut context, &tx, None, Some(&[3, 4, 8]));
            assert!(context.verify_tx(&invalid, PERF_LIMIT).is_err());
            assert!(context
                .verify_tx(
                    &tx.as_advanced_builder().set_witnesses(vec![]).build(),
                    PERF_LIMIT
                )
                .is_err());
        }
    }
}
