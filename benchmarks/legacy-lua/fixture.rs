// One-off performance fixture for PR #75 at af622ac.
// Included at the end of tests/src/tests/mutant_rust.rs in an isolated checkout.
const PERF_REPEATS: usize = 5;
const PERF_LIMIT: u64 = 500_000_000;
const PERF_LUA: &str = include_str!("../../../examples/mutant-rust/equipment-merge.lua");

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
    let cycles = context.verify_tx(tx, PERF_LIMIT).expect("benchmark transaction");
    for _ in 1..PERF_REPEATS {
        assert_eq!(context.verify_tx(tx, PERF_LIMIT).unwrap(), cycles);
    }
    cycles
}

fn perf_rule(context: &mut Context, native: bool, ordinal: usize) -> (CellDep, [u8; 32], u64) {
    let contract = if native { "spore_extension_rust" } else { "spore_extension_lua" };
    let (code, dep) = build_spore_contract_materials(context, contract);
    let input = perf_input(context, 1, ordinal);
    let id = build_type_id(&input, 0);
    let script = context.build_script_with_hash_type(
        &code,
        if native { ScriptHashType::Data2 } else { ScriptHashType::Data1 },
        id.to_vec().into(),
    ).unwrap();
    let output = build_normal_output_cell_with_type(context, Some(script));
    let data = if native { binary("mutant_equipment") } else { PERF_LUA.as_bytes().to_vec().into() };
    let mut builder = TransactionBuilder::default().input(input).output(output.clone())
        .output_data(data.pack()).cell_dep(dep);
    if !native {
        let (_, lib) = build_spore_contract_materials(context, "libckblua.so");
        builder = builder.cell_dep(lib);
    }
    let tx = context.complete_tx(builder.build());
    let cycles = perf_measure(context, &tx);
    let point = perf_point(2, ordinal);
    context.create_cell_with_out_point(point.clone(), output, data);
    (CellDep::new_builder().out_point(point).build(), id, cycles)
}

fn perf_mint(context: &mut Context, rules: &[(CellDep, [u8; 32])], extra: &[CellDep]) -> TransactionView {
    let ids = rules.iter().map(|(_, id)| hex::encode(id)).collect::<Vec<_>>().join(",");
    let mime = if rules.is_empty() { "application/x.equipment".to_owned() }
        else { format!("application/x.equipment;mutant[]={ids}") };
    let input = perf_input(context, 3, 0);
    let id = build_type_id(&input, 0);
    let (code, dep) = build_spore_contract_materials(context, "spore");
    let script = build_spore_type_script(context, &code, id.to_vec().into()).unwrap();
    let data = build_serialized_spore_data(vec![3, 4, 7], &mime, None);
    let output = build_normal_output_cell_with_type(context, Some(script.clone()));
    let tx = TransactionBuilder::default().input(input).output(output)
        .output_data(data.as_slice().pack()).cell_dep(dep)
        .cell_deps(rules.iter().map(|(dep, _)| dep.clone()).collect::<Vec<_>>())
        .cell_deps(extra.to_vec()).build();
    let action = co_build::build_mint_spore_action(context, id, data.as_slice());
    context.complete_tx(co_build::complete_co_build_message_with_actions(tx, &[(Some(script), action)]))
}

#[test]
fn pr75_performance_report() {
    println!("PERF_METHOD repeats={PERF_REPEATS} limit={PERF_LIMIT} content=3,4,7 payment=none lock=ALWAYS_SUCCESS");
    for (label, kinds) in [
        ("baseline", vec![]),
        ("rust1", vec![true]), ("lua1", vec![false]),
        ("rust2", vec![true; 2]), ("lua2", vec![false; 2]),
        ("rust4", vec![true; 4]), ("lua4", vec![false; 4]),
        ("mixed2", vec![true, false]),
    ] {
        let mut context = Context::new_with_deterministic_rng();
        let mut rules = Vec::new();
        let mut extra = Vec::new();
        for (ordinal, native) in kinds.iter().copied().enumerate() {
            let (dep, id, cycles) = perf_rule(&mut context, native, ordinal);
            if ordinal == 0 {
                println!("PERF creation {} {cycles}", if native { "rust" } else { "lua" });
            }
            rules.push((dep, id));
        }
        if kinds.contains(&false) {
            for name in ["spore_extension_lua", "libckblua.so"] {
                extra.push(build_spore_contract_materials(&mut context, name).1);
            }
        }
        let tx = perf_mint(&mut context, &rules, &extra);
        let mint_cycles = perf_measure(&context, &tx);
        let transfer = move_spore(&mut context, &tx, false);
        let burn = move_spore(&mut context, &tx, true);
        let transfer_cycles = perf_measure(&context, &transfer);
        let burn_cycles = perf_measure(&context, &burn);
        println!("PERF {label} mint={mint_cycles} transfer={transfer_cycles} burn={burn_cycles} deps={} tx_bytes={}",
            tx.cell_deps().len(), tx.data().as_slice().len());
        // Both implementations must reject the same invalid recipe under full Spore validation.
        if !rules.is_empty() {
            let invalid = replace_content(&mut context, &tx, None, Some(&[3, 4, 8]));
            assert!(context.verify_tx(&invalid, PERF_LIMIT).is_err());
            assert!(context.verify_tx(&tx.as_advanced_builder().set_witnesses(vec![]).build(), PERF_LIMIT).is_err());
        }
    }
}
