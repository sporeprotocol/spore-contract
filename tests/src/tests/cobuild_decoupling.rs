use crate::{utils::*, MAX_CYCLES};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{TransactionBuilder, TransactionView},
        packed::*,
        prelude::*,
    },
    context::Context,
};

fn store(context: &mut Context, tx: &TransactionView) -> (CellDep, Bytes, Script) {
    let output = tx.outputs().get(0).unwrap();
    let script = output.type_().to_opt().unwrap();
    let data = tx.outputs_data().get(0).unwrap().raw_data();
    let point = context.create_cell(output, data.clone());
    (
        CellDep::new_builder().out_point(point).build(),
        data,
        script,
    )
}
fn verify_no_intent(context: &Context, tx: &TransactionView, label: &str) {
    assert_eq!(tx.witnesses().len(), 0, "{label}");
    context
        .verify_tx(tx, MAX_CYCLES)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    let opaque = tx
        .as_advanced_builder()
        .witness(vec![0xffu8, 0, 0x81].pack())
        .build();
    context
        .verify_tx(&opaque, MAX_CYCLES)
        .unwrap_or_else(|e| panic!("opaque witness {label}: {e:?}"));
}
fn move_and_burn(context: &mut Context, minted: &TransactionView, name: &str, allow_burn: bool) {
    let output = minted.outputs().get(0).unwrap();
    let data = minted.outputs_data().get(0).unwrap();
    let input = CellInput::new_builder()
        .previous_output(context.create_cell(output.clone(), data.raw_data()))
        .build();
    let destination = output.lock().as_builder().args(vec![17].pack()).build();
    let output = output.as_builder().lock(destination).build();
    let transfer = context.complete_tx(
        TransactionBuilder::default()
            .input(input.clone())
            .output(output)
            .output_data(data.clone())
            .cell_deps(minted.cell_deps())
            .build(),
    );
    verify_no_intent(context, &transfer, &format!("{name} transfer"));
    let mut changed = data.raw_data().to_vec();
    changed.push(9);
    let invalid = transfer
        .as_advanced_builder()
        .set_outputs_data(vec![changed.pack()])
        .build();
    assert!(
        context.verify_tx(&invalid, MAX_CYCLES).is_err(),
        "immutable {name}"
    );
    let normal = build_normal_output(context);
    let burn = context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(normal)
            .output_data(Bytes::new().pack())
            .cell_deps(minted.cell_deps())
            .build(),
    );
    if allow_burn {
        verify_no_intent(context, &burn, &format!("{name} burn"));
    } else {
        assert!(
            context.verify_tx(&burn, MAX_CYCLES).is_err(),
            "cluster burn remains forbidden"
        );
    }
}
#[test]
fn decoupling_all_eleven_operations_keep_state_rules() {
    let mut context = Context::default();
    let spore = build_single_spore_mint_tx(
        &mut context,
        vec![1, 2, 3],
        "application/x.cobuild-test",
        None,
        None,
    );
    let spore = context.complete_tx(spore.as_advanced_builder().set_witnesses(vec![]).build());
    verify_no_intent(&context, &spore, "Spore mint");
    move_and_burn(&mut context, &spore, "Spore", true);

    let (cluster_code, cluster_code_dep) = build_spore_contract_materials(&mut context, "cluster");
    let input = build_normal_input(&mut context);
    let cluster_id = build_type_id(&input, 0);
    let cluster_script =
        build_spore_type_script(&mut context, &cluster_code, cluster_id.to_vec().into());
    let cluster_data = build_serialized_cluster_data("optional intent", "without mandatory intent");
    let cluster_output = build_normal_output_cell_with_type(&mut context, cluster_script);
    let cluster = context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(cluster_output)
            .output_data(cluster_data.as_slice().pack())
            .cell_dep(cluster_code_dep)
            .build(),
    );
    verify_no_intent(&context, &cluster, "Cluster mint");
    move_and_burn(&mut context, &cluster, "Cluster", false);
    let (cluster_dep, _, _) = store(&mut context, &cluster);

    let (proxy_code, proxy_code_dep) =
        build_spore_contract_materials(&mut context, "cluster_proxy");
    let input = build_normal_input(&mut context);
    let proxy_id = build_type_id(&input, 0);
    let mut proxy_args = proxy_id.to_vec();
    proxy_args.extend_from_slice(&0u64.to_le_bytes());
    let proxy_script = build_spore_type_script(&mut context, &proxy_code, proxy_args.into());
    let proxy_output = build_normal_output_cell_with_type(&mut context, proxy_script);
    let proxy = context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(proxy_output)
            .output_data(cluster_id.to_vec().pack())
            .cell_deps(vec![proxy_code_dep, cluster_dep])
            .build(),
    );
    verify_no_intent(&context, &proxy, "Proxy mint");
    move_and_burn(&mut context, &proxy, "Proxy", true);
    let (proxy_dep, _, proxy_script) = store(&mut context, &proxy);

    let (agent_code, agent_code_dep) =
        build_spore_contract_materials(&mut context, "cluster_agent");
    let input = build_normal_input(&mut context);
    let agent_script =
        build_spore_type_script(&mut context, &agent_code, cluster_id.to_vec().into());
    let output = build_normal_output_cell_with_type(&mut context, agent_script);
    let agent = context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(output)
            .output_data(proxy_script.calc_script_hash().as_slice().pack())
            .cell_deps(vec![agent_code_dep, proxy_dep])
            .build(),
    );
    verify_no_intent(&context, &agent, "Agent mint");
    move_and_burn(&mut context, &agent, "Agent", true);
}

#[test]
fn optional_intent_is_not_consensus_authority() {
    let mut context = Context::default();
    let tx = build_single_spore_mint_tx(&mut context, vec![1, 2, 3], "text/plain", None, None);
    let tx = context.complete_tx(tx);
    // Existing, well-formed CoBuild messages still work.
    context.verify_tx(&tx, MAX_CYCLES).unwrap();
    let bare = tx.as_advanced_builder().set_witnesses(vec![]).build();
    verify_no_intent(&context, &bare, "Spore mint");
    let script = tx.outputs().get(0).unwrap().type_().to_opt();
    // The ID and data hash deliberately contradict the real mint. Consumers
    // must verify the actual transaction before trusting or displaying intent.
    let false_action = co_build::build_mint_spore_action(&mut context, [99; 32], &[99]);
    let misleading =
        co_build::complete_co_build_message_with_actions(bare.clone(), &[(script, false_action)]);
    context.verify_tx(&misleading, MAX_CYCLES).unwrap();
    // An intent cannot make an invalid real state acceptable.
    let empty = build_serialized_spore_data(vec![], "text/plain", None);
    for candidate in [&bare, &misleading] {
        let invalid = candidate
            .as_advanced_builder()
            .set_outputs_data(vec![empty.as_slice().pack()])
            .build();
        assert!(context.verify_tx(&invalid, MAX_CYCLES).is_err());
    }
    let output = bare.outputs().get(0).unwrap();
    let wrong_id = output
        .type_()
        .to_opt()
        .unwrap()
        .as_builder()
        .args(vec![0u8; 32].pack())
        .build();
    let invalid = bare
        .as_advanced_builder()
        .set_outputs(vec![output
            .as_builder()
            .type_(Some(wrong_id).pack())
            .build()])
        .build();
    assert!(context.verify_tx(&invalid, MAX_CYCLES).is_err());
    let duplicate = bare
        .as_advanced_builder()
        .output(bare.outputs().get(0).unwrap())
        .output_data(bare.outputs_data().get(0).unwrap())
        .build();
    assert!(context.verify_tx(&duplicate, MAX_CYCLES).is_err());
}

#[test]
fn lua_mutant_rules_apply_without_intent() {
    for (code, allowed) in [
        ("ckb.exit_script(0)", true),
        ("ckb.exit_script(101)", false),
    ] {
        let mut context = Context::default();
        let (deps, mutant_id) = build_single_mutant_celldep_tx(&mut context, code, 1);
        let mime = format!("text/plain;mutant[]={}", hex::encode(mutant_id));
        let tx = build_single_spore_mint_tx(&mut context, vec![1], &mime, None, None);
        let tx = context.complete_tx(
            tx.as_advanced_builder()
                .set_witnesses(vec![])
                .cell_deps(deps.cell_deps())
                .build(),
        );
        assert_eq!(
            context.verify_tx(&tx, MAX_CYCLES).is_ok(),
            allowed,
            "{code}"
        );
    }
}

#[test]
fn malformed_proxy_and_agent_cells_are_rejected_without_intent() {
    for (contract, args_len, data_len) in [
        ("cluster_proxy", 32, 31),
        ("cluster_proxy", 32, 33),
        ("cluster_proxy", 31, 32),
        ("cluster_proxy", 33, 32),
        ("cluster_agent", 32, 31),
        ("cluster_agent", 32, 33),
        ("cluster_agent", 31, 32),
        ("cluster_agent", 40, 32),
    ] {
        let mut context = Context::default();
        let (code, dep) = build_spore_contract_materials(&mut context, contract);
        let script = build_spore_type_script(&mut context, &code, vec![1; args_len].into());
        let data = vec![2; data_len];
        let input = build_agent_proxy_input(&mut context, &data, script.clone());
        let output = build_normal_output_cell_with_type(&mut context, script);
        let transfer = context.complete_tx(
            TransactionBuilder::default()
                .input(input.clone())
                .output(output)
                .output_data(data.pack())
                .cell_dep(dep.clone())
                .build(),
        );
        assert!(
            context.verify_tx(&transfer, MAX_CYCLES).is_err(),
            "{contract} transfer args={args_len} data={data_len}"
        );
        let output = build_normal_output(&mut context);
        let burn = context.complete_tx(
            TransactionBuilder::default()
                .input(input)
                .output(output)
                .output_data(Bytes::new().pack())
                .cell_dep(dep)
                .build(),
        );
        assert!(
            context.verify_tx(&burn, MAX_CYCLES).is_err(),
            "{contract} burn args={args_len} data={data_len}"
        );
    }
}

#[test]
fn real_sighash_lock_still_requires_signatures_and_commits_to_intent() {
    use ckb_sdk::{
        traits::SecpCkbRawKeySigner,
        types::ScriptGroup,
        unlock::{ScriptSigner, SecpSighashScriptSigner},
    };
    use ckb_system_scripts::BUNDLED_CELL;
    use ckb_testtool::ckb_hash::blake2b_256;
    use ckb_testtool::ckb_types::core::ScriptHashType;
    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    // Public test fixture, never a wallet key.
    let key = SecretKey::from_slice(&[42; 32]).unwrap();
    let pubkey = PublicKey::from_secret_key(&Secp256k1::new(), &key);
    let args = Bytes::copy_from_slice(&blake2b_256(pubkey.serialize())[..20]);
    let signer =
        SecpSighashScriptSigner::new(Box::new(SecpCkbRawKeySigner::new_with_secret_keys(vec![
            key,
        ])));
    let mut context = Context::default();
    let lock_code = context.deploy_cell(Bytes::from(
        BUNDLED_CELL
            .get("specs/cells/secp256k1_blake160_sighash_all")
            .unwrap()
            .into_owned(),
    ));
    let secp_data = context.deploy_cell(Bytes::from(
        BUNDLED_CELL
            .get("specs/cells/secp256k1_data")
            .unwrap()
            .into_owned(),
    ));
    let lock = context
        .build_script_with_hash_type(&lock_code, ScriptHashType::Data, args)
        .unwrap();
    let input_point = context.create_cell(
        CellOutput::new_builder()
            .lock(lock.clone())
            .capacity((1000 * CAPACITY_UNIT).pack())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_point)
        .build();
    let id = build_type_id(&input, 0);
    let (code, dep) = build_spore_contract_materials(&mut context, "spore");
    let script = build_spore_type_script(&mut context, &code, id.to_vec().into());
    let output = build_normal_output_cell_with_type(&mut context, script.clone());
    let data = build_serialized_spore_data(vec![1, 2, 3], "text/plain", None);
    let tx = context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(output)
            .output_data(data.as_slice().pack())
            .cell_deps(vec![
                dep,
                CellDep::new_builder().out_point(lock_code).build(),
                CellDep::new_builder().out_point(secp_data).build(),
            ])
            .witness(WitnessArgs::default().as_bytes().pack())
            .build(),
    );
    let mut group = ScriptGroup::from_lock_script(&lock);
    group.input_indices.push(0);
    let action = co_build::build_mint_spore_action(&mut context, id, data.as_slice());
    let with_intent =
        co_build::complete_co_build_message_with_actions(tx.clone(), &[(script, action)]);
    for unsigned in [tx, with_intent] {
        assert!(
            context.verify_tx(&unsigned, MAX_CYCLES).is_err(),
            "missing signature"
        );
        let signed = signer.sign_tx(&unsigned, &group).unwrap();
        context.verify_tx(&signed, MAX_CYCLES).unwrap();
        let bad_sig = WitnessArgs::new_builder()
            .lock(Some(Bytes::from(vec![0; 65])).pack())
            .build();
        let mut witnesses: Vec<_> = signed.witnesses().into_iter().collect();
        witnesses[0] = bad_sig.as_bytes().pack();
        assert!(
            context
                .verify_tx(
                    &signed
                        .as_advanced_builder()
                        .set_witnesses(witnesses)
                        .build(),
                    MAX_CYCLES
                )
                .is_err(),
            "invalid signature"
        );
        // Changing the real recipient after signing still fails at the lock.
        let output = signed.outputs().get(0).unwrap();
        let changed_lock = output.lock().as_builder().args(vec![7].pack()).build();
        let changed_output = output.as_builder().lock(changed_lock).build();
        let tampered = signed
            .as_advanced_builder()
            .set_outputs(vec![changed_output])
            .build();
        assert!(
            context.verify_tx(&tampered, MAX_CYCLES).is_err(),
            "recipient tampering"
        );
        if signed.witnesses().len() > 1 {
            let mut witnesses: Vec<_> = signed.witnesses().into_iter().collect();
            witnesses[1] = vec![0xff].pack();
            assert!(
                context
                    .verify_tx(
                        &signed
                            .as_advanced_builder()
                            .set_witnesses(witnesses)
                            .build(),
                        MAX_CYCLES
                    )
                    .is_err(),
                "intent tampering after signing"
            );
        }
    }
}

#[test]
fn cluster_authority_and_proxy_payment_remain_required_without_intent() {
    let mut context = Context::default();
    let (cluster_code, _) = build_spore_contract_materials(&mut context, "cluster");
    let cluster_data = build_serialized_cluster_data("protected", "authority test");
    let (cluster_id, _, _, _, cluster_dep) =
        build_cluster_materials(&mut context, &cluster_code, cluster_data, 0, &[9]);
    let spore =
        build_single_spore_mint_tx(&mut context, vec![1], "text/plain", None, Some(cluster_id));
    let bare = spore.as_advanced_builder().set_witnesses(vec![]).build();
    let missing = context.complete_tx(bare.clone());
    assert!(
        context.verify_tx(&missing, MAX_CYCLES).is_err(),
        "missing cluster"
    );
    let unauthorized = context.complete_tx(
        bare.as_advanced_builder()
            .cell_dep(cluster_dep.clone())
            .build(),
    );
    assert!(
        context.verify_tx(&unauthorized, MAX_CYCLES).is_err(),
        "missing cluster owner authority"
    );

    let (proxy_code, proxy_code_dep) =
        build_spore_contract_materials(&mut context, "cluster_proxy");
    let input = build_normal_input(&mut context);
    let id = build_type_id(&input, 0);
    let proxy_type = build_spore_type_script_with_payment(&mut context, &proxy_code, &id, 1);
    let output = build_normal_output_cell_with_type(&mut context, proxy_type.clone());
    let unauthorized = context.complete_tx(
        TransactionBuilder::default()
            .input(input)
            .output(output)
            .output_data(cluster_id.to_vec().pack())
            .cell_deps(vec![proxy_code_dep, cluster_dep])
            .build(),
    );
    assert!(
        context.verify_tx(&unauthorized, MAX_CYCLES).is_err(),
        "proxy creation still needs cluster owner authority"
    );

    // A protected proxy charges one CKB; none of the normal input/output
    // capacity belongs to its owner, so the unpaid Agent must fail.
    let proxy_dep =
        build_normal_cell_dep_with_lock_args(&mut context, &cluster_id, proxy_type.clone(), &[9]);
    let proxy_owner = context.get_cell(&proxy_dep.out_point()).unwrap().0.lock();
    let (agent_code, agent_code_dep) =
        build_spore_contract_materials(&mut context, "cluster_agent");
    let agent_type = build_spore_type_script(&mut context, &agent_code, cluster_id.to_vec().into());
    let input = build_normal_input(&mut context);
    let output = build_normal_output_cell_with_type(&mut context, agent_type);
    let proxy_hash = proxy_type.unwrap().calc_script_hash();
    let unsigned = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(proxy_hash.as_slice().pack())
        .cell_dep(agent_code_dep)
        .build();
    let missing = context.complete_tx(unsigned.clone());
    assert!(
        context.verify_tx(&missing, MAX_CYCLES).is_err(),
        "missing referenced proxy"
    );
    let unpaid = context.complete_tx(unsigned.as_advanced_builder().cell_dep(proxy_dep).build());
    assert!(
        context.verify_tx(&unpaid, MAX_CYCLES).is_err(),
        "unpaid proxy fee"
    );
    let fee = CellOutput::new_builder()
        .lock(proxy_owner)
        .capacity(CAPACITY_UNIT.pack())
        .build();
    let paid = context.complete_tx(
        unpaid
            .as_advanced_builder()
            .output(fee)
            .output_data(Bytes::new().pack())
            .build(),
    );
    context
        .verify_tx(&paid, MAX_CYCLES)
        .expect("valid payment without intent");
}
