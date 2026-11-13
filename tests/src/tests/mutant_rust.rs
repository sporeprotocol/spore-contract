//! Native execution and continuation tests use the real Meepo VM scheduler.
use crate::{utils::*, Loader, MAX_CYCLES};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::*,
        prelude::*,
    },
    context::Context,
};
#[allow(unused_imports)]
use molecule::prelude::{Builder as SporeBuilder, Entity as SporeEntity};
use spore_types::generated::spore::SporeData;

fn binary(name: &str) -> Bytes {
    Loader::default().load_binary(name)
}

fn rejects(context: &Context, tx: &TransactionView, fragment: &str) {
    let error = format!(
        "{:?}",
        context.verify_tx(tx, MAX_CYCLES).expect_err("must reject")
    );
    assert!(error.contains(fragment), "expected {fragment}, got {error}");
}

fn creation(
    context: &mut Context,
    code: Bytes,
    mode: &[u8],
    payment: Option<u64>,
) -> TransactionView {
    let (contract, dep) = build_spore_contract_materials(context, "spore_extension_rust");
    let input = build_normal_input(context);
    let id = build_type_id(&input, 0);
    let mut args = id.to_vec();
    if let Some(payment) = payment {
        args.extend_from_slice(&payment.to_le_bytes());
    }
    let script = context
        .build_script_with_hash_type(&contract, ScriptHashType::Type, args.into())
        .unwrap();
    let lock_code = context.deploy_cell(ckb_testtool::builtin::ALWAYS_SUCCESS.clone());
    let lock = context
        .build_script(&lock_code, mode.to_vec().into())
        .unwrap();
    let output = build_normal_output_cell_with_type(context, Some(script))
        .as_builder()
        .lock(lock)
        .build();
    context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(output)
            .output_data(code.pack())
            .cell_dep(dep)
            .build(),
    )
}

fn rule(
    context: &mut Context,
    name: &str,
    mode: &[u8],
    payment: Option<u64>,
) -> (CellDep, [u8; 32]) {
    let tx = creation(context, binary(name), mode, payment);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("native Mutant creation");
    let output = tx.outputs().get(0).unwrap();
    let id = output.type_().to_opt().unwrap().args().raw_data()[..32]
        .try_into()
        .unwrap();
    let point = context.create_cell(output, tx.outputs_data().get(0).unwrap().raw_data());
    (CellDep::new_builder().out_point(point).build(), id)
}

fn mint(context: &mut Context, rules: &[(CellDep, [u8; 32])], content: &[u8]) -> TransactionView {
    let ids = rules
        .iter()
        .map(|(_, id)| hex::encode(id))
        .collect::<Vec<_>>()
        .join(",");
    let tx = build_single_spore_mint_tx(
        context,
        content.to_vec(),
        &format!("application/x.equipment;mutant[]={ids}"),
        None,
        None,
    );
    let tx = tx
        .as_advanced_builder()
        .cell_deps(rules.iter().map(|(dep, _)| dep.clone()).collect::<Vec<_>>())
        .build();
    context.complete_tx(tx)
}

fn move_spore(context: &mut Context, minted: &TransactionView, burn: bool) -> TransactionView {
    let output = minted.outputs().get(0).unwrap();
    let data = minted.outputs_data().get(0).unwrap();
    let script = output.type_().to_opt().unwrap();
    let id = script.args().raw_data()[..32].try_into().unwrap();
    let input = CellInput::new_builder()
        .previous_output(context.create_cell(output.clone(), data.raw_data()))
        .build();
    let tx = TransactionBuilder::default()
        .input(input)
        .cell_deps(minted.cell_deps());
    let tx = if burn {
        tx.build()
    } else {
        tx.output(output).output_data(data).build()
    };
    let action = if burn {
        co_build::build_burn_spore_action(context, id)
    } else {
        co_build::build_transfer_spore_action(context, id)
    };
    co_build::complete_co_build_message_with_actions(tx, &[(Some(script), action)])
}

fn replace_content(
    context: &mut Context,
    tx: &TransactionView,
    mime: Option<&str>,
    content: Option<&[u8]>,
) -> TransactionView {
    let original = SporeData::from_slice(&tx.outputs_data().get(0).unwrap().raw_data()).unwrap();
    let mut builder = original.as_builder();
    if let Some(mime) = mime {
        builder = builder.content_type(mime.as_bytes().into());
    }
    if let Some(content) = content {
        builder = builder.content(content.into());
    }
    let data = builder.build();
    let script = tx.outputs().get(0).unwrap().type_().to_opt().unwrap();
    let id = script.args().raw_data()[..32].try_into().unwrap();
    let action = co_build::build_mint_spore_action(context, id, data.as_slice());
    let tx = tx
        .as_advanced_builder()
        .set_outputs_data(vec![data.as_slice().pack()])
        .set_witnesses(vec![])
        .build();
    co_build::complete_co_build_message_with_actions(tx, &[(Some(script), action)])
}

#[test]
fn rust_mutant_executes_without_lua_or_receipt_and_is_deterministic() {
    let mut context = Context::default();
    let rule = rule(&mut context, "mutant_equipment", b"", None);
    let tx = mint(&mut context, &[rule], &[3, 4, 7]);
    assert_eq!(tx.outputs().len(), 1);
    let cycles = context.verify_tx(&tx, MAX_CYCLES).unwrap();
    for _ in 0..3 {
        assert_eq!(context.verify_tx(&tx, MAX_CYCLES).unwrap(), cycles);
    }
    println!("native Rust Mutant [3,4,7]: {cycles} transaction cycles");
}

#[test]
fn native_recipe_rejects_invalid_zero_overflow_and_oversized_content() {
    for content in [
        vec![3, 4, 8],
        vec![0, 4, 4],
        vec![255, 1, 0],
        vec![3, 4],
        vec![1; 4097],
    ] {
        let mut context = Context::default();
        let rule = rule(&mut context, "mutant_equipment", b"", None);
        let tx = mint(&mut context, &[rule], &content);
        rejects(&context, &tx, "error code 93");
    }
}

#[test]
fn native_creation_rejects_invalid_elf_wrong_id_args_and_vm_version() {
    let mut context = Context::default();
    let invalid = creation(&mut context, Bytes::from_static(b"not an ELF"), b"", None);
    rejects(&context, &invalid, "error code 91");
    let valid = creation(&mut context, binary("mutant_equipment"), b"", None);
    for (args, hash_type, expected) in [
        (vec![0; 32], ScriptHashType::Type, "error code 19"),
        (vec![0; 31], ScriptHashType::Type, "error code 20"),
        (vec![0; 41], ScriptHashType::Type, "error code 20"),
        (vec![0; 32], ScriptHashType::Data1, "error code 90"),
        (vec![0; 32], ScriptHashType::Data2, "error code 90"),
    ] {
        let output = valid.outputs().get(0).unwrap();
        let script = output
            .type_()
            .to_opt()
            .unwrap()
            .as_builder()
            .args(args.pack())
            .hash_type(hash_type.into())
            .code_hash(if hash_type == ScriptHashType::Type {
                output.type_().to_opt().unwrap().code_hash()
            } else {
                calc_code_hash(binary("spore_extension_rust")).pack()
            })
            .build();
        let tx = valid
            .as_advanced_builder()
            .set_outputs(vec![output.as_builder().type_(Some(script).pack()).build()])
            .build();
        rejects(&context, &tx, expected);
    }
}

#[test]
fn native_creation_runs_validator_and_propagates_nonzero_status() {
    let mut context = Context::default();
    let tx = creation(&mut context, binary("mutant_probe"), b"create", None);
    rejects(&context, &tx, "error code 93");
    // ELF-looking garbage still must pass the VM loader and execute successfully.
    let mut garbage = vec![0; 128];
    garbage[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    garbage[16..20].copy_from_slice(&[2, 0, 243, 0]);
    let tx = creation(&mut context, garbage.into(), b"", None);
    assert!(context.verify_tx(&tx, MAX_CYCLES).is_err());
}

#[test]
fn native_mutant_code_is_immutable_and_cannot_be_destroyed_or_duplicated() {
    let mut context = Context::default();
    let created = creation(&mut context, binary("mutant_equipment"), b"", None);
    context.verify_tx(&created, MAX_CYCLES).unwrap();
    let output = created.outputs().get(0).unwrap();
    let data = created.outputs_data().get(0).unwrap();
    let input = CellInput::new_builder()
        .previous_output(context.create_cell(output.clone(), data.raw_data()))
        .build();
    let tx = TransactionBuilder::default()
        .input(input)
        .output(output.clone())
        .output_data(data)
        .cell_deps(created.cell_deps())
        .build();
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("unchanged Mutant transfer");
    let changed = tx
        .as_advanced_builder()
        .set_outputs_data(vec![binary("mutant_probe").pack()])
        .build();
    rejects(&context, &changed, "error code 15");
    let destroyed = tx
        .as_advanced_builder()
        .set_outputs(vec![])
        .set_outputs_data(vec![])
        .build();
    rejects(&context, &destroyed, "error code 18");
    let duplicated = tx
        .as_advanced_builder()
        .output(output)
        .output_data(tx.outputs_data().get(0).unwrap())
        .build();
    rejects(&context, &duplicated, "error code 18");
}

#[test]
fn native_transfer_burn_and_spore_immutability() {
    let mut context = Context::default();
    let rule = rule(&mut context, "mutant_equipment", b"", None);
    let tx = mint(&mut context, &[rule], &[3, 4, 7]);
    context.verify_tx(&tx, MAX_CYCLES).unwrap();
    let transfer = move_spore(&mut context, &tx, false);
    context
        .verify_tx(&transfer, MAX_CYCLES)
        .expect("native transfer");
    let data = SporeData::from_slice(&transfer.outputs_data().get(0).unwrap().raw_data())
        .unwrap()
        .as_builder()
        .content([3u8, 4, 8].as_slice().into())
        .build();
    let changed = transfer
        .as_advanced_builder()
        .set_outputs_data(vec![data.as_slice().pack()])
        .build();
    rejects(&context, &changed, "error code 61");
    let burn = move_spore(&mut context, &tx, true);
    context.verify_tx(&burn, MAX_CYCLES).expect("native burn");
}

#[test]
fn native_rules_can_reject_each_spore_operation() {
    for mode in [b"mint".as_slice(), b"transfer", b"burn"] {
        let mut context = Context::default();
        let rule = rule(&mut context, "mutant_probe", mode, None);
        let tx = mint(&mut context, &[rule], &[3, 4, 7]);
        let target = if mode == b"mint" {
            tx
        } else {
            context.verify_tx(&tx, MAX_CYCLES).unwrap();
            move_spore(&mut context, &tx, mode == b"burn")
        };
        rejects(&context, &target, "error code 93");
    }
}

#[test]
fn every_native_mutant_runs_in_declared_order() {
    let mut context = Context::default();
    let first = rule(&mut context, "mutant_probe", b"", None);
    let second = rule(&mut context, "mutant_probe", b"mint", None);
    let tx = mint(&mut context, &[first.clone(), second.clone()], &[3, 4, 7]);
    rejects(&context, &tx, "error code 93");
    let tx = mint(&mut context, &[second, first], &[3, 4, 7]);
    rejects(&context, &tx, "error code 93");
}

#[test]
fn successful_mutant_never_skips_cobuild_on_any_operation() {
    let mut context = Context::default();
    let rule = rule(&mut context, "mutant_equipment", b"", None);
    let mint = mint(&mut context, &[rule], &[3, 4, 7]);
    for tx in [
        mint.clone(),
        move_spore(&mut context, &mint, false),
        move_spore(&mut context, &mint, true),
    ] {
        context.verify_tx(&tx, MAX_CYCLES).unwrap();
        let missing = tx.as_advanced_builder().set_witnesses(vec![]).build();
        assert!(
            context.verify_tx(&missing, MAX_CYCLES).is_err(),
            "missing co-build accepted"
        );
    }
}

#[test]
fn successful_mutant_never_skips_multipart_validation() {
    let mut context = Context::default();
    let rule = rule(&mut context, "mutant_probe", b"", None);
    let tx = mint(&mut context, &[rule.clone()], b"bad multipart");
    for (suffix, error) in [("", "error code 64"), (";boundary=abc", "error code 69")] {
        let mime = format!("multipart/mixed;mutant[]={}{suffix}", hex::encode(rule.1));
        let invalid = replace_content(&mut context, &tx, Some(&mime), None);
        rejects(&context, &invalid, error);
    }
}

#[test]
fn native_dependency_type_hash_and_vm_binding_are_exact() {
    let mut context = Context::default();
    let (dep, id) = rule(&mut context, "mutant_equipment", b"", None);
    let (output, data) = context.get_cell(&dep.out_point()).unwrap();
    let original = output.type_().to_opt().unwrap();
    for script in [
        original
            .clone()
            .as_builder()
            .hash_type(ScriptHashType::Data1.into())
            .build(),
        original
            .clone()
            .as_builder()
            .code_hash([9u8; 32].pack())
            .build(),
        original
            .clone()
            .as_builder()
            .args(vec![0u8; 31].pack())
            .build(),
        original
            .clone()
            .as_builder()
            .args(vec![0u8; 41].pack())
            .build(),
        original
            .clone()
            .as_builder()
            .args(vec![0u8; 32].pack())
            .build(),
    ] {
        let output = output
            .clone()
            .as_builder()
            .type_(Some(script).pack())
            .build();
        let dep = CellDep::new_builder()
            .out_point(context.create_cell(output, data.clone()))
            .build();
        let tx = mint(&mut context, &[(dep, id)], &[3, 4, 7]);
        rejects(&context, &tx, "error code 71");
    }
    let tx = mint(&mut context, &[(dep.clone(), id)], &[3, 4, 7]);
    let without_rule = tx
        .as_advanced_builder()
        .set_cell_deps(
            tx.cell_deps()
                .into_iter()
                .filter(|candidate| candidate != &dep)
                .collect(),
        )
        .build();
    rejects(&context, &without_rule, "error code 71");
}

#[test]
fn native_mutants_require_type_bound_spore_code() {
    let mut context = Context::default();
    let rule = rule(&mut context, "mutant_equipment", b"", None);
    let tx = mint(&mut context, &[rule], &[3, 4, 7]);
    let output = tx.outputs().get(0).unwrap();
    let script = output
        .type_()
        .to_opt()
        .unwrap()
        .as_builder()
        .hash_type(ScriptHashType::Data1.into())
        .code_hash(calc_code_hash(binary("spore")).pack())
        .build();
    let tx = tx
        .as_advanced_builder()
        .set_outputs(vec![output.as_builder().type_(Some(script).pack()).build()])
        .build();
    rejects(&context, &tx, "error code 90");
}

#[test]
fn native_code_at_dependency_and_output_indices_above_255_executes() {
    let mut context = Context::default();
    let native = rule(&mut context, "mutant_equipment", b"", None);
    let input = build_normal_input(&mut context);
    let index = 260;
    let id = build_type_id(&input, index);
    let (code, code_dep) = build_spore_contract_materials(&mut context, "spore");
    let script = build_spore_type_script(&mut context, &code, id.to_vec().into()).unwrap();
    let data = build_serialized_spore_data(
        vec![3, 4, 7],
        &format!("application/x.equipment;mutant[]={}", hex::encode(native.1)),
        None,
    );
    let mut builder = TransactionBuilder::default().input(input);
    for n in 0..260u32 {
        let output = build_normal_output(&mut context);
        builder = builder
            .output(output.clone())
            .output_data(Bytes::new().pack());
        let point = context.create_cell(output, n.to_le_bytes().to_vec().into());
        builder = builder.cell_dep(CellDep::new_builder().out_point(point).build());
    }
    let output = build_normal_output_cell_with_type(&mut context, Some(script.clone()));
    let tx = builder
        .output(output)
        .output_data(data.as_slice().pack())
        .cell_dep(code_dep)
        .cell_dep(native.0)
        .build();
    let action = co_build::build_mint_spore_action(&mut context, id, data.as_slice());
    let tx = context.complete_tx(co_build::complete_co_build_message_with_actions(
        tx,
        &[(Some(script), action)],
    ));
    context
        .verify_tx(&tx, 30_000_000)
        .expect("full-width indices");
}

#[test]
fn native_child_infinite_loop_exhausts_shared_cycle_budget() {
    let mut context = Context::default();
    let rule = rule(&mut context, "mutant_probe", b"loop", None);
    let tx = mint(&mut context, &[rule], &[3, 4, 7]);
    let error = context
        .verify_tx(&tx, 10_000_000)
        .expect_err("child loop exhausts budget");
    assert!(format!("{error:?}").contains("ExceededMaximumCycles"));
}

#[test]
fn native_payment_is_aggregated_for_all_mutants_and_checked_for_overflow() {
    let mut context = Context::default();
    let a = rule(&mut context, "mutant_equipment", b"payee", Some(100));
    let b = rule(&mut context, "mutant_equipment", b"payee", Some(200));
    let tx = mint(&mut context, &[a.clone(), b], &[3, 4, 7]);
    rejects(&context, &tx, "error code 72");
    let (payee, _) = context.get_cell(&a.0.out_point()).unwrap();
    let output = CellOutput::new_builder()
        .capacity(300u64.pack())
        .lock(payee.lock())
        .build();
    let enough = tx
        .as_advanced_builder()
        .output(output.clone())
        .output_data(Bytes::new().pack())
        .build();
    context
        .verify_tx(&enough, MAX_CYCLES)
        .expect("sum of both payments");
    let short = tx
        .as_advanced_builder()
        .output(output.as_builder().capacity(299u64.pack()).build())
        .output_data(Bytes::new().pack())
        .build();
    rejects(&context, &short, "error code 72");
    let huge = rule(&mut context, "mutant_equipment", b"", Some(u64::MAX));
    let overflow = mint(&mut context, &[huge], &[3, 4, 7]);
    rejects(&context, &overflow, "error code 94");
}

// A historical Lua dependency is deliberately synthetic: the new contracts
// must reject it by lifecycle hash, without loading or executing an interpreter.
fn legacy_lua_rule(context: &mut Context) -> (CellDep, [u8; 32]) {
    // data_hash recorded in deployment/migration/testnet/spore_extension_lua/
    // 2024-03-16-0.2.2-beta.2.json.
    let hash: [u8; 32] =
        hex::decode("5ff1a403458b436ea4b2ceb72f1fa70a6507968493315b646f5302661cb68e57")
            .unwrap()
            .try_into()
            .unwrap();
    let id = [0x5a; 32];
    let script = Script::new_builder()
        .code_hash(hash.pack())
        .hash_type(ScriptHashType::Data1.into())
        .args(id.to_vec().pack())
        .build();
    let output = build_normal_output_cell_with_type(context, Some(script));
    let point = context.create_cell(output, Bytes::from_static(b"assert(true)"));
    (CellDep::new_builder().out_point(point).build(), id)
}

#[test]
fn legacy_lua_dependencies_are_rejected_on_mint_transfer_and_burn() {
    let mut context = Context::default();
    let legacy = legacy_lua_rule(&mut context);
    let mint = mint(&mut context, &[legacy], &[3, 4, 7]);
    for tx in [
        mint.clone(),
        move_spore(&mut context, &mint, false),
        move_spore(&mut context, &mint, true),
    ] {
        rejects(&context, &tx, "error code 71");
    }
}

#[test]
fn mixed_native_and_legacy_rules_are_rejected_in_either_order() {
    let mut context = Context::default();
    let native = rule(&mut context, "mutant_equipment", b"", None);
    let legacy = legacy_lua_rule(&mut context);
    for rules in [
        [native.clone(), legacy.clone()],
        [legacy.clone(), native.clone()],
    ] {
        let tx = mint(&mut context, &rules, &[3, 4, 7]);
        rejects(&context, &tx, "error code 71");
    }
}

fn cluster_creation(
    context: &mut Context,
    native: &(CellDep, [u8; 32]),
) -> (TransactionView, [u8; 32]) {
    let (code, dep) = build_spore_contract_materials(context, "cluster");
    let input = build_normal_input(context);
    let id = build_type_id(&input, 0);
    assert_ne!(
        id, native.1,
        "cluster and Mutant identities must be independent"
    );
    let script = build_spore_type_script(context, &code, id.to_vec().into()).unwrap();
    let data = build_serialized_cluster_data("native cluster", "Rust rule")
        .as_builder()
        .mutant_id(native.1.as_slice().into())
        .build();
    let output = build_normal_output_cell_with_type(context, Some(script.clone()));
    let action = co_build::build_mint_cluster_action(context, id, data.as_slice());
    let tx = TransactionBuilder::default()
        .input(input)
        .output(output.clone())
        .output_data(data.as_slice().pack())
        .cell_deps(vec![dep.clone(), native.0.clone()])
        .build();
    let tx = context.complete_tx(co_build::complete_co_build_message_with_actions(
        tx,
        &[(Some(script), action)],
    ));
    (tx, id)
}

fn cluster(context: &mut Context, native: &(CellDep, [u8; 32])) -> (CellDep, [u8; 32]) {
    let (tx, id) = cluster_creation(context, native);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("cluster uses declared native Mutant ID, including payment args");
    let output = tx.outputs().get(0).unwrap();
    let data = tx.outputs_data().get(0).unwrap().raw_data();
    let dep = tx.cell_deps().get(0).unwrap();
    let missing = context.complete_tx(tx.as_advanced_builder().set_cell_deps(vec![dep]).build());
    rejects(context, &missing, "error code 54");
    let point = context.create_cell(output, data);
    (CellDep::new_builder().out_point(point).build(), id)
}

fn clustered_mint(
    context: &mut Context,
    native: &(CellDep, [u8; 32]),
    cluster: &(CellDep, [u8; 32]),
) -> TransactionView {
    let tx = build_single_spore_mint_tx(
        context,
        vec![3, 4, 7],
        &format!("application/x.equipment;mutant[]={}", hex::encode(native.1)),
        None,
        Some(cluster.1),
    );
    context.complete_tx(
        tx.as_advanced_builder()
            .cell_deps(vec![native.0.clone(), cluster.0.clone()])
            .build(),
    )
}

#[test]
fn native_cluster_creation_looks_up_mutant_id_and_accepts_payment_args() {
    let mut context = Context::default();
    let native = rule(&mut context, "mutant_equipment", b"", Some(0));
    let cluster = cluster(&mut context, &native);
    let tx = clustered_mint(&mut context, &native, &cluster);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("native rule inside native cluster");
}

#[test]
fn successful_mutant_never_skips_cluster_dependency_or_ownership() {
    let mut context = Context::default();
    let native = rule(&mut context, "mutant_equipment", b"", None);
    let cluster = cluster(&mut context, &native);
    let tx = clustered_mint(&mut context, &native, &cluster);
    context.verify_tx(&tx, MAX_CYCLES).unwrap();
    let no_cluster = tx
        .as_advanced_builder()
        .set_cell_deps(
            tx.cell_deps()
                .into_iter()
                .filter(|dep| dep != &cluster.0)
                .collect(),
        )
        .build();
    rejects(&context, &no_cluster, "error code 6");
    let (output, data) = context.get_cell(&cluster.0.out_point()).unwrap();
    let code = context.deploy_cell(ckb_testtool::builtin::ALWAYS_SUCCESS.clone());
    let lock = context
        .build_script(&code, Bytes::from_static(b"other owner"))
        .unwrap();
    let cluster = (
        CellDep::new_builder()
            .out_point(context.create_cell(output.as_builder().lock(lock).build(), data))
            .build(),
        cluster.1,
    );
    let no_authorization = clustered_mint(&mut context, &native, &cluster);
    rejects(&context, &no_authorization, "error code 7");
}

#[test]
fn successful_mutant_never_skips_cluster_required_mutant() {
    let mut context = Context::default();
    let a = rule(&mut context, "mutant_equipment", b"", None);
    let b = rule(&mut context, "mutant_equipment", b"", None);
    let cluster = cluster(&mut context, &b);
    let tx = clustered_mint(&mut context, &a, &cluster);
    rejects(&context, &tx, "error code 73");
}

#[test]
fn native_success_does_not_bypass_input_locks_or_cobuild_fields() {
    let mut context = Context::default();
    let native = rule(&mut context, "mutant_equipment", b"", None);
    let tx = mint(&mut context, &[native], &[3, 4, 7]);
    let script = tx.outputs().get(0).unwrap().type_().to_opt().unwrap();
    let action = co_build::build_mint_spore_action(
        &mut context,
        [0; 32],
        &tx.outputs_data().get(0).unwrap().raw_data(),
    );
    let invalid = co_build::complete_co_build_message_with_actions(
        tx.as_advanced_builder().set_witnesses(vec![]).build(),
        &[(Some(script), action)],
    );
    rejects(&context, &invalid, "error code 12");
    let (code, dep) = build_spore_contract_materials(&mut context, "mutant_equipment");
    let lock = build_spore_type_script(&mut context, &code, Bytes::new()).unwrap();
    let output = build_normal_output(&mut context)
        .as_builder()
        .lock(lock)
        .build();
    let input = CellInput::new_builder()
        .previous_output(context.create_cell(output, Bytes::new()))
        .build();
    let invalid = tx.as_advanced_builder().input(input).cell_dep(dep).build();
    rejects(&context, &invalid, "Inputs[1].Lock");
}

#[test]
fn native_creation_bounds_code_size_and_requires_riscv_elf64() {
    let mut context = Context::default();
    for (size, machine) in [(256 * 1024 + 1, 243u8), (128, 62u8)] {
        let mut code = vec![0; size];
        code[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        code[16..20].copy_from_slice(&[2, 0, machine, 0]);
        let tx = creation(&mut context, code.into(), b"", None);
        rejects(&context, &tx, "error code 91");
    }
}

#[test]
fn cluster_creation_rejects_a_legacy_lua_mutant() {
    let mut context = Context::default();
    let legacy = legacy_lua_rule(&mut context);
    let (tx, _) = cluster_creation(&mut context, &legacy);
    rejects(&context, &tx, "error code 54");
}

#[path = "mutant_performance.rs"]
mod performance;
