use ckb_testtool::ckb_hash::blake2b_256;
use ckb_testtool::ckb_types::core::TransactionView;
use ckb_testtool::ckb_types::packed;
use ckb_testtool::ckb_types::prelude::*;

use ckb_testtool::context::Context;
use spore_types::generated::action::{
    Address, AddressUnion, AgentCreate, AgentTransfer, Burn, Byte32, Bytes, ClusterCreate,
    ClusterTransfer, Mint, ProxyCreate, ProxyTransfer, Script, SporeAction, SporeActionUnion,
    Transfer,
};
use spore_utils::co_build_types::{
    Action, ActionVec, Message, SighashAll, WitnessLayout, WitnessLayoutUnion,
};

use super::internal;

fn h256_to_byte32(hash: [u8; 32]) -> Byte32 {
    let hash = hash
        .into_iter()
        .map(packed::Byte::new)
        .collect::<Vec<packed::Byte>>()
        .try_into()
        .unwrap();
    Byte32::new_builder().set(hash).build()
}

fn script_to_address(script: packed::Script) -> Address {
    let code_hash = script.code_hash().unpack();
    let hash_type = script.hash_type();
    let args = script.args().raw_data();

    let code_hash = h256_to_byte32(code_hash.into());
    let args = Bytes::new_builder()
        .set(args.into_iter().map(packed::Byte::new).collect())
        .build();

    let script = Script::new_builder()
        .code_hash(code_hash)
        .hash_type(hash_type)
        .args(args)
        .build();

    Address::new_builder()
        .set(AddressUnion::Script(script))
        .build()
}

pub fn complete_co_build_message_with_actions(
    tx: TransactionView,
    actions: &[(Option<packed::Script>, SporeActionUnion)],
) -> TransactionView {
    let action_value_vec = actions
        .to_owned()
        .into_iter()
        .map(|(script_hash, action)| {
            let script_hash = if let Some(script_hash) = script_hash {
                script_hash.calc_script_hash()
            } else {
                packed::Byte32::default()
            };
            let spore_action = SporeAction::new_builder().set(action).build();
            Action::new_builder()
                .script_hash(script_hash)
                .data(spore_action.as_slice().pack())
                .build()
        })
        .collect();
    let action_vec = ActionVec::new_builder().set(action_value_vec).build();
    let message = Message::new_builder().actions(action_vec).build();
    let sighash_all = SighashAll::new_builder().message(message).build();
    let witness_layout = WitnessLayout::new_builder()
        .set(WitnessLayoutUnion::SighashAll(sighash_all))
        .build();

    tx.as_advanced_builder()
        .witness(witness_layout.as_slice().pack())
        .build()
}

pub fn build_mint_action(
    context: &mut Context,
    nft_id: [u8; 32],
    content: &[u8],
) -> SporeActionUnion {
    let to = internal::build_always_success_script(context);
    let mint = Mint::new_builder()
        .spore_id(h256_to_byte32(nft_id))
        .data_hash(h256_to_byte32(blake2b_256(content)))
        .to(script_to_address(to))
        .build();
    SporeActionUnion::Mint(mint)
}

pub fn build_transfer_action(context: &mut Context, nft_id: [u8; 32]) -> SporeActionUnion {
    let script = internal::build_always_success_script(context);
    let address = script_to_address(script);
    let transfer = Transfer::new_builder()
        .spore_id(h256_to_byte32(nft_id))
        .from(address.clone())
        .to(address)
        .build();
    SporeActionUnion::Transfer(transfer)
}

pub fn build_burn_action(context: &mut Context, nft_id: [u8; 32]) -> SporeActionUnion {
    let from = internal::build_always_success_script(context);
    let burn = Burn::new_builder()
        .spore_id(h256_to_byte32(nft_id))
        .from(script_to_address(from))
        .build();
    SporeActionUnion::Burn(burn)
}

pub fn build_cluster_create_action(
    context: &mut Context,
    cluster_id: [u8; 32],
    content: &[u8],
) -> SporeActionUnion {
    let to = internal::build_always_success_script(context);
    let cluster_create = ClusterCreate::new_builder()
        .cluster_id(h256_to_byte32(cluster_id))
        .data_hash(h256_to_byte32(blake2b_256(content)))
        .to(script_to_address(to))
        .build();
    SporeActionUnion::ClusterCreate(cluster_create)
}

pub fn build_cluster_transfer_action(
    context: &mut Context,
    cluster_id: [u8; 32],
) -> SporeActionUnion {
    let script = internal::build_always_success_script(context);
    let address = script_to_address(script);
    let cluster_transfer = ClusterTransfer::new_builder()
        .cluster_id(h256_to_byte32(cluster_id))
        .from(address.clone())
        .to(address)
        .build();
    SporeActionUnion::ClusterTransfer(cluster_transfer)
}

pub fn build_proxy_create_action(
    context: &mut Context,
    cluster_id: [u8; 32],
    proxy_id: [u8; 32],
) -> SporeActionUnion {
    let to = internal::build_always_success_script(context);
    let proxy_create = ProxyCreate::new_builder()
        .cluster_id(h256_to_byte32(cluster_id))
        .proxy_id(h256_to_byte32(proxy_id))
        .to(script_to_address(to))
        .build();
    SporeActionUnion::ProxyCreate(proxy_create)
}

pub fn build_proxy_transfer_action(
    context: &mut Context,
    cluster_id: [u8; 32],
    proxy_id: [u8; 32],
) -> SporeActionUnion {
    let script = internal::build_always_success_script(context);
    let from = script_to_address(script);
    let to = from.clone();
    let proxy_transfer = ProxyTransfer::new_builder()
        .cluster_id(h256_to_byte32(cluster_id))
        .proxy_id(h256_to_byte32(proxy_id))
        .from(from)
        .to(to)
        .build();
    SporeActionUnion::ProxyTransfer(proxy_transfer)
}

pub fn build_agent_create_action(context: &mut Context, cluster_id: [u8; 32]) -> SporeActionUnion {
    let to = internal::build_always_success_script(context);
    let agent_create = AgentCreate::new_builder()
        .cluster_id(h256_to_byte32(cluster_id))
        .to(script_to_address(to))
        .build();
    SporeActionUnion::AgentCreate(agent_create)
}

pub fn build_agent_transfer_action(
    context: &mut Context,
    cluster_id: [u8; 32],
) -> SporeActionUnion {
    let script = internal::build_always_success_script(context);
    let from = script_to_address(script);
    let to = from.clone();
    let agent_transfer = AgentTransfer::new_builder()
        .cluster_id(h256_to_byte32(cluster_id))
        .from(from)
        .to(to)
        .build();
    SporeActionUnion::AgentTransfer(agent_transfer)
}
