use crate::{utils::*, MAX_CYCLES};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        h256,
        packed::*,
        prelude::*,
    },
    context::Context,
};
use molecule::prelude::Entity as MoleculeEntity;

#[derive(Clone)]
struct Creation {
    agent: Script,
    agent_dep: CellDep,
    proxy: Script,
    proxy_dep: CellDep,
    cluster_id: [u8; 32],
    proxy_id: [u8; 32],
}

fn paid_proxy(context: &mut Context, proxy: Script, cluster_id: &[u8; 32]) -> CellDep {
    build_normal_cell_dep_with_lock_args(context, cluster_id, Some(proxy), &[9])
}

fn verify(
    context: &mut Context,
    creations: &[Creation],
    payment: u64,
) -> Result<TransactionView, String> {
    let funding = build_normal_input(context);
    let beneficiary = context
        .get_cell(&creations[0].proxy_dep.out_point())
        .unwrap()
        .0
        .lock();
    let mut tx = TransactionBuilder::default().input(funding);
    let mut deps = Vec::new();
    let mut actions = Vec::new();
    for creation in creations {
        tx = tx
            .output(build_normal_output_cell_with_type(
                context,
                Some(creation.agent.clone()),
            ))
            .output_data(creation.proxy.calc_script_hash().as_slice().pack());
        for dep in [&creation.agent_dep, &creation.proxy_dep] {
            if !deps.contains(dep) {
                deps.push(dep.clone());
            }
        }
        actions.push((
            Some(creation.agent.clone()),
            co_build::build_mint_agent_action(context, creation.cluster_id, creation.proxy_id),
        ));
    }
    let fee = CellOutput::new_builder()
        .lock(beneficiary)
        .capacity((payment * CAPACITY_UNIT).pack())
        .build();
    let tx = tx
        .output(fee)
        .output_data(Bytes::new().pack())
        .cell_deps(deps)
        .build();
    let tx = co_build::complete_co_build_message_with_actions(tx, &actions);
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .map(|_| tx)
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn distinct_paid_proxies_require_combined_capacity_for_the_same_beneficiary() {
    let mut context = Context::default();
    let (agent_point, agent_dep) = build_spore_contract_materials(&mut context, "cluster_agent");
    let (proxy_point, _) = build_spore_contract_materials(&mut context, "cluster_proxy");
    let creations = [1, 2].map(|index| {
        let cluster_id = [index; 32];
        let proxy_id = [index + 2; 32];
        let agent = build_spore_type_script(&mut context, &agent_point, cluster_id.to_vec().into())
            .unwrap();
        let proxy = build_spore_type_script_with_payment(&mut context, &proxy_point, &proxy_id, 80)
            .unwrap();
        let proxy_dep = paid_proxy(&mut context, proxy.clone(), &cluster_id);
        Creation {
            agent,
            agent_dep: agent_dep.clone(),
            proxy,
            proxy_dep,
            cluster_id,
            proxy_id,
        }
    });
    verify(&mut context, &creations[..1], 80).unwrap();
    verify(&mut context, &creations[1..], 80).unwrap();
    assert!(verify(&mut context, &creations, 80)
        .unwrap_err()
        .contains("error code 44"));
    verify(&mut context, &creations, 160).unwrap();
}

fn historical_paid_agent(context: &mut Context, cluster_id: [u8; 32]) -> Creation {
    let legacy_binary = include_bytes!("../../fixtures/published-testnet/cluster_agent.bin");
    let legacy_point = context.deploy_cell(Bytes::from_static(legacy_binary));
    let legacy_agent = context
        .build_script_with_hash_type(
            &legacy_point,
            ScriptHashType::Data1,
            cluster_id.to_vec().into(),
        )
        .unwrap();
    let legacy_id = [2; 32];
    let legacy_proxy = Script::new_builder()
        .code_hash(
            h256!("0x4349889bda064adab8f49f7dd8810d217917f7df28e9b2a1df0b74442399670a").pack(),
        )
        .hash_type(ScriptHashType::Data1.into())
        .args(
            [legacy_id.as_slice(), &(80 * CAPACITY_UNIT).to_le_bytes()]
                .concat()
                .pack(),
        )
        .build();
    Creation {
        agent: legacy_agent,
        agent_dep: CellDep::new_builder().out_point(legacy_point).build(),
        proxy_dep: paid_proxy(context, legacy_proxy.clone(), &cluster_id),
        proxy: legacy_proxy,
        cluster_id,
        proxy_id: legacy_id,
    }
}

#[test]
fn historical_and_type_bound_agents_share_the_payment_boundary() {
    if !crate::Loader::default().is_testnet() {
        return;
    }
    let mut context = Context::default();
    let (agent_point, agent_dep) = build_spore_contract_materials(&mut context, "cluster_agent");
    let (proxy_point, _) = build_spore_contract_materials(&mut context, "cluster_proxy");
    let cluster_id = [1; 32];
    let legacy = historical_paid_agent(&mut context, cluster_id);
    let proxy_id = [3; 32];
    let proxy =
        build_spore_type_script_with_payment(&mut context, &proxy_point, &proxy_id, 80).unwrap();
    let current = Creation {
        agent: build_spore_type_script(&mut context, &agent_point, cluster_id.to_vec().into())
            .unwrap(),
        agent_dep,
        proxy_dep: paid_proxy(&mut context, proxy.clone(), &cluster_id),
        proxy,
        cluster_id,
        proxy_id,
    };
    verify(&mut context, &[legacy.clone()], 80).unwrap();
    verify(&mut context, &[current.clone()], 80).unwrap();
    let different_proxies = [legacy.clone(), current.clone()];
    assert!(verify(&mut context, &different_proxies, 80)
        .unwrap_err()
        .contains("error code 44"));
    verify(&mut context, &different_proxies, 160).unwrap();

    let same_proxy = Creation {
        proxy: legacy.proxy.clone(),
        proxy_dep: legacy.proxy_dep.clone(),
        proxy_id: legacy.proxy_id,
        ..current
    };
    verify(&mut context, &[same_proxy.clone()], 80).unwrap();
    assert!(verify(&mut context, &[legacy, same_proxy], 160)
        .unwrap_err()
        .contains("error code 47"));
}

#[test]
fn v3_spore_mint_uses_the_published_testnet_cluster_and_agent_lineage() {
    if !crate::Loader::default().is_testnet() {
        return;
    }
    let mut context = Context::default();
    let cluster_id = [1; 32];
    let legacy = historical_paid_agent(&mut context, cluster_id);
    let creation = verify(&mut context, std::slice::from_ref(&legacy), 80)
        .expect("historical Agent creation with the published Proxy identity");
    let (agent_output, agent_data) = creation.output_with_data(0).unwrap();
    let agent_point = context.create_cell(agent_output.clone(), agent_data.clone());

    // This is the published v0.2.2-beta.2 Cluster code hash. The Cluster is
    // referenced as a read-only dependency, so its binary does not execute.
    let cluster = Script::new_builder()
        .code_hash(
            h256!("0x0bbe768b519d8ea7b96d58f1182eb7e6ef96c541fbd9526975077ee09f049058").pack(),
        )
        .hash_type(ScriptHashType::Data1.into())
        .args(cluster_id.as_slice().pack())
        .build();
    let cluster_data = build_serialized_cluster_data("Pudge Cluster", "v0.2.2-beta.2");
    let cluster_dep = build_normal_cell_dep(&mut context, cluster_data.as_slice(), Some(cluster));

    let transfer = co_build::build_transfer_agent_action(&mut context, cluster_id);
    let mint = build_single_spore_mint_tx_with_extra_action(
        &mut context,
        vec![1],
        "text/plain",
        None,
        Some(cluster_id),
        vec![(Some(legacy.agent), transfer)],
    );
    let tx = mint
        .as_advanced_builder()
        .input(
            CellInput::new_builder()
                .previous_output(agent_point)
                .build(),
        )
        .output(agent_output)
        .output_data(agent_data.pack())
        .cell_dep(legacy.agent_dep)
        .cell_dep(cluster_dep)
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("V3 Spore mint authorized by transfer of a historical Agent");
}
