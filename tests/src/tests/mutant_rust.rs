use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{TransactionBuilder, TransactionView},
        packed::{CellDep, CellInput},
        prelude::*,
    },
    context::Context,
};

use crate::{utils::*, Loader, MAX_CYCLES};

const LUA_RECIPE: &str = include_str!("../../../examples/mutant-rust/equipment-merge.lua");
const RUST_BRIDGE: &str = include_str!("../../../examples/mutant-rust/require-rust.lua");

// A genuine Spore mint, with the same content and Lua Mutant interface for both
// implementations. Some(binary) selects the opt-in Rust companion example.
fn mint(context: &mut Context, content: &[u8], rust: Option<Bytes>) -> TransactionView {
    let lua = match &rust {
        Some(binary) => {
            RUST_BRIDGE.replace("@CODE_HASH@", &hex::encode(calc_code_hash(binary.clone())))
        }
        None => LUA_RECIPE.to_owned(),
    };
    let (mutant_deps, id) = build_single_mutant_celldep_tx(context, &lua, 0);
    let tx = build_single_spore_mint_tx(
        context,
        content.to_vec(),
        &format!("application/x.spore-equipment;mutant[]={}", hex::encode(id)),
        None,
        None,
    );
    let mut builder = tx.as_advanced_builder().cell_deps(mutant_deps.cell_deps());
    if let Some(binary) = rust {
        let outpoint = context.deploy_cell(binary);
        let target = tx
            .outputs()
            .get(0)
            .unwrap()
            .type_()
            .to_opt()
            .unwrap()
            .calc_script_hash();
        let receipt_type = build_spore_type_script(context, &outpoint, target.as_bytes());
        let receipt = build_normal_output_cell_with_type(context, receipt_type);
        builder = builder
            .output(receipt)
            .output_data(Bytes::new().pack())
            .cell_dep(CellDep::new_builder().out_point(outpoint).build());
    }
    context.complete_tx(builder.build())
}

fn rust_binary() -> Bytes {
    Loader::default().load_binary("mutant_equipment")
}

fn rejects(context: &Context, tx: &TransactionView, expected: &str) {
    let error = context.verify_tx(tx, MAX_CYCLES).expect_err("must reject");
    assert!(format!("{error:?}").contains(expected), "{error:?}");
}

#[test]
fn equipment_lua_and_rust_execution_and_cycles() {
    for (name, binary) in [("lua", None), ("rust-companion", Some(rust_binary()))] {
        let mut context = Context::default();
        let tx = mint(&mut context, &[3, 4, 7], binary);
        let cycles = context.verify_tx(&tx, MAX_CYCLES).expect("valid recipe");
        assert_eq!(cycles, context.verify_tx(&tx, MAX_CYCLES).unwrap());
        println!("equipment [3,4,7] {name}: {cycles} total transaction cycles");
        let error = context
            .verify_tx(&tx, cycles - 1)
            .expect_err("cycle budget enforced");
        assert!(
            format!("{error:?}").contains("ExceededMaximumCycles"),
            "{error:?}"
        );
    }
}

#[test]
fn equipment_invalid_recipes_fail_in_both_languages() {
    for content in [
        &[3, 4, 8][..],
        &[0, 4, 4],
        &[255, 1, 0],
        &[3, 4],
        &[3, 4, 7, 0],
    ] {
        for binary in [None, Some(rust_binary())] {
            let mut context = Context::default();
            let native = binary.is_some();
            let tx = mint(&mut context, content, binary);
            rejects(
                &context,
                &tx,
                if native {
                    "error code 9"
                } else {
                    "error code -1"
                },
            );
        }
    }
}

#[test]
fn equipment_boundary_recipe_is_valid() {
    for binary in [None, Some(rust_binary())] {
        let mut context = Context::default();
        let tx = mint(&mut context, &[254, 1, 255], binary);
        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("non-overflowing byte sum");
    }
}

#[test]
fn rust_bridge_requires_an_output_not_just_a_dependency() {
    let mut context = Context::default();
    let tx = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
    let tx = tx
        .as_advanced_builder()
        .set_outputs(vec![tx.outputs().get(0).unwrap()])
        .set_outputs_data(vec![tx.outputs_data().get(0).unwrap()])
        .build();
    rejects(&context, &tx, "error code -1");
}

#[test]
fn rust_bridge_rejects_wrong_target_or_backend() {
    for wrong_backend in [false, true] {
        let mut context = Context::default();
        let tx = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
        let receipt = tx.outputs().get(1).unwrap();
        let script = receipt.type_().to_opt().unwrap();
        let script = if wrong_backend {
            // Data2 can execute this ELF, but is not the pinned Data1 backend.
            // Data (VM V0) could fail while loading the ELF before the bridge
            // runs, making an exact bridge-error assertion order-dependent.
            script.as_builder().hash_type(4u8.into()).build()
        } else {
            script.as_builder().args(vec![0u8; 32].pack()).build()
        };
        let receipt = receipt.as_builder().type_(Some(script).pack()).build();
        let tx = tx
            .as_advanced_builder()
            .set_outputs(vec![tx.outputs().get(0).unwrap(), receipt])
            .build();
        let error = context
            .verify_tx(&tx, MAX_CYCLES)
            .expect_err("wrong binding");
        let error = format!("{error:?}");
        // Script-group order is hash-dependent: either the bridge rejects the
        // binding or the companion rejects the nonexistent target first.
        assert!(
            error.contains("error code -1") || (!wrong_backend && error.contains("error code 7")),
            "{error}"
        );
    }
}

#[test]
fn rust_bridge_rejects_a_different_code_hash() {
    let mut context = Context::default();
    let tx = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
    let receipt = tx.outputs().get(1).unwrap();
    // ALWAYS_SUCCESS is already a code dependency for the normal input lock.
    // Its type group succeeds, so only the bridge can reject this substitution.
    let substitute = calc_code_hash(ckb_testtool::builtin::ALWAYS_SUCCESS.clone()).pack();
    let script = receipt
        .type_()
        .to_opt()
        .unwrap()
        .as_builder()
        .code_hash(substitute)
        .build();
    let receipt = receipt.as_builder().type_(Some(script).pack()).build();
    let tx = tx
        .as_advanced_builder()
        .set_outputs(vec![tx.outputs().get(0).unwrap(), receipt])
        .build();
    rejects(&context, &tx, "error code -1");
}

#[test]
fn rust_receipt_spending_and_recreation_do_not_cache_validation() {
    let mut context = Context::default();
    let minted = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
    context.verify_tx(&minted, MAX_CYCLES).unwrap();
    let receipt = minted.outputs().get(1).unwrap();
    let input = CellInput::new_builder()
        .previous_output(context.create_cell(receipt.clone(), Bytes::new()))
        .build();

    // An empty-data receipt is disposable; spending it grants no asset rights.
    let spend = TransactionBuilder::default()
        .input(input.clone())
        .cell_deps(minted.cell_deps())
        .build();
    context
        .verify_tx(&spend, MAX_CYCLES)
        .expect("receipt disposal");

    let recreated = spend
        .as_advanced_builder()
        .output(receipt)
        .output_data(Bytes::new().pack())
        .build();
    rejects(&context, &recreated, "error code 7");

    // Use the previously valid receipt as an input to a new invalid mint. Its
    // old target must not satisfy the bridge's binding to the new Spore.
    let invalid = mint(&mut context, &[3, 4, 8], Some(rust_binary()));
    let invalid = invalid
        .as_advanced_builder()
        .input(input)
        .set_outputs(vec![invalid.outputs().get(0).unwrap()])
        .set_outputs_data(vec![invalid.outputs_data().get(0).unwrap()])
        .build();
    rejects(&context, &invalid, "error code -1");
}

#[test]
fn rust_invalid_binary_is_not_a_native_success() {
    let mut context = Context::default();
    let tx = mint(
        &mut context,
        &[3, 4, 7],
        Some(Bytes::from_static(b"not an ELF")),
    );
    let error = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("VM rejects invalid ELF");
    assert!(format!("{error:?}").contains("ElfParseError"), "{error:?}");
}

#[test]
fn rust_receipt_rejects_data_and_duplicate_outputs() {
    for duplicate in [false, true] {
        let mut context = Context::default();
        let tx = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
        let tx = if duplicate {
            tx.as_advanced_builder()
                .output(tx.outputs().get(1).unwrap())
                .output_data(Bytes::new().pack())
                .build()
        } else {
            tx.as_advanced_builder()
                .set_outputs_data(vec![tx.outputs_data().get(0).unwrap(), vec![1u8].pack()])
                .build()
        };
        rejects(&context, &tx, "error code 6");
    }
}

#[test]
fn rust_target_data_is_bounded() {
    let mut context = Context::default();
    let tx = mint(&mut context, &vec![1; 4097], Some(rust_binary()));
    rejects(&context, &tx, "error code 8");
}

#[test]
fn recipe_mutants_can_be_created_normally() {
    for lua in [
        LUA_RECIPE.to_owned(),
        RUST_BRIDGE.replace("@CODE_HASH@", &hex::encode(calc_code_hash(rust_binary()))),
    ] {
        let mut context = Context::default();
        let (_, library) = build_spore_contract_materials(&mut context, "libckblua.so");
        let (code, dep) = build_spore_contract_materials(&mut context, "spore_extension_lua");
        let input = build_normal_input(&mut context);
        let id = build_type_id(&input, 0);
        let script = build_spore_type_script(&mut context, &code, id.to_vec().into());
        let output = build_normal_output_cell_with_type(&mut context, script);
        let tx = context.complete_tx(
            TransactionBuilder::default()
                .input(input)
                .output(output)
                .output_data(lua.pack())
                .cell_deps(vec![library, dep])
                .build(),
        );
        context
            .verify_tx(&tx, MAX_CYCLES)
            .expect("ordinary Lua Mutant creation");
    }
}

#[test]
fn equipment_transfer_melt_and_immutability() {
    for binary in [None, Some(rust_binary())] {
        let mut context = Context::default();
        let minted = mint(&mut context, &[3, 4, 7], binary);
        context.verify_tx(&minted, MAX_CYCLES).unwrap();
        let output = minted.outputs().get(0).unwrap();
        let data = minted.outputs_data().get(0).unwrap();
        let script = output.type_().to_opt().unwrap();
        let id: [u8; 32] = script.args().raw_data().as_ref().try_into().unwrap();
        let input = CellInput::new_builder()
            .previous_output(context.create_cell(output.clone(), data.raw_data()))
            .build();
        let transfer = TransactionBuilder::default()
            .input(input.clone())
            .output(output)
            .output_data(data)
            .cell_deps(minted.cell_deps())
            .build();
        let action = co_build::build_transfer_spore_action(&mut context, id);
        let transfer = co_build::complete_co_build_message_with_actions(
            transfer,
            &[(Some(script.clone()), action)],
        );
        context
            .verify_tx(&transfer, MAX_CYCLES)
            .expect("transfer needs no receipt");

        let original = spore_types::generated::spore::SporeData::from_slice(
            &transfer.outputs_data().get(0).unwrap().raw_data(),
        )
        .unwrap();
        let modified = original
            .as_builder()
            .content([3u8, 4, 8].as_slice().into())
            .build();
        let changed = transfer
            .as_advanced_builder()
            .set_outputs_data(vec![modified.as_slice().pack()])
            .build();
        rejects(&context, &changed, "error code 61");

        let melt = TransactionBuilder::default()
            .input(input)
            .cell_deps(minted.cell_deps())
            .build();
        let action = co_build::build_burn_spore_action(&mut context, id);
        let melt =
            co_build::complete_co_build_message_with_actions(melt, &[(Some(script), action)]);
        context
            .verify_tx(&melt, MAX_CYCLES)
            .expect("melt needs no receipt");
    }
}

#[test]
fn rust_receipt_does_not_bypass_input_locks() {
    let mut context = Context::default();
    let tx = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
    let (code, dep) = build_spore_contract_materials(&mut context, "mutant_equipment");
    // This deliberately rejecting lock returns Arguments (5). Output validation
    // through Lua must never turn an unauthorized input into a valid spend.
    let lock = build_spore_type_script(&mut context, &code, Bytes::new()).unwrap();
    let output = build_normal_output(&mut context)
        .as_builder()
        .lock(lock)
        .build();
    let input = CellInput::new_builder()
        .previous_output(context.create_cell(output, Bytes::new()))
        .build();
    let tx = tx.as_advanced_builder().input(input).cell_dep(dep).build();
    rejects(&context, &tx, "Inputs[1].Lock");
}

#[test]
fn native_arguments_missing_target_and_malformed_molecule_fail() {
    let mut context = Context::default();
    let tx = mint(&mut context, &[3, 4, 7], Some(rust_binary()));
    let receipt = tx.outputs().get(1).unwrap();
    let only_receipt = tx
        .as_advanced_builder()
        .set_outputs(vec![receipt.clone()])
        .set_outputs_data(vec![Bytes::new().pack()])
        .build();
    rejects(&context, &only_receipt, "error code 7");
    let script = receipt
        .type_()
        .to_opt()
        .unwrap()
        .as_builder()
        .args(Bytes::new().pack())
        .build();
    let bad_args = only_receipt
        .as_advanced_builder()
        .set_outputs(vec![receipt
            .as_builder()
            .type_(Some(script).pack())
            .build()])
        .build();
    rejects(&context, &bad_args, "error code 5");

    // Isolate native Molecule parsing without Spore rejecting the data first.
    let target_type = build_normal_output(&mut context).lock();
    let target = build_normal_output_cell_with_type(&mut context, Some(target_type.clone()));
    let receipt = tx.outputs().get(1).unwrap();
    let script = receipt
        .type_()
        .to_opt()
        .unwrap()
        .as_builder()
        .args(target_type.calc_script_hash().as_bytes().pack())
        .build();
    let receipt = receipt.as_builder().type_(Some(script).pack()).build();
    let malformed = tx
        .as_advanced_builder()
        .set_outputs(vec![target, receipt])
        .set_outputs_data(vec![vec![0u8; 16].pack(), Bytes::new().pack()])
        .build();
    rejects(&context, &malformed, "error code 8");
}

#[test]
fn lua_infinite_loop_exhausts_transaction_budget() {
    let mut context = Context::default();
    let (deps, id) = build_single_mutant_celldep_tx(
        &mut context,
        "if spore_ext_mode == 1 then while true do end end",
        0,
    );
    let tx = build_single_spore_mint_tx(
        &mut context,
        vec![3, 4, 7],
        &format!("application/x.spore-equipment;mutant[]={}", hex::encode(id)),
        None,
        None,
    );
    let tx = context.complete_tx(tx.as_advanced_builder().cell_deps(deps.cell_deps()).build());
    rejects(&context, &tx, "ExceededMaximumCycles");
}
