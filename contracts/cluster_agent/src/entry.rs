// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::ckb_constants::Source::{self, CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell_data, load_cell_lock_hash, load_cell_type, QueryIter};
use ckb_std::{ckb_types::prelude::*, debug, high_level::load_script};

use spore_errors::error::Error;
use spore_types::generated::action;
use spore_utils::{
    calc_capacity_sum, find_position_by_type, find_position_by_type_hash, load_self_id,
};
use spore_utils::{
    check_spore_address, extract_spore_action, CLUSTER_PROXY_ID_LEN,
    CLUSTER_PROXY_ID_WITH_PAYMENT_LEN,
};

fn is_valid_cluster_proxy_cell(script_hash: &[u8; 32]) -> bool {
    crate::hash::CLUSTER_PROXY_CODE_HASHES.contains(script_hash)
}

fn has_conflict_agent(source: Source, cell_data: &[u8]) -> bool {
    let script = load_script().unwrap_or_default();
    let self_code_hash = script.code_hash();
    let agents_count = QueryIter::new(load_cell_type, source)
        .enumerate()
        .filter(|(index, type_)| {
            if let Some(type_) = type_ {
                if type_.code_hash().as_slice() == self_code_hash.as_slice() {
                    let data = load_cell_data(*index, source).unwrap();
                    return cell_data == data;
                }
            }
            false
        })
        .count();
    agents_count > 1
}

fn process_creation(_index: usize) -> Result<(), Error> {
    let proxy_type_hash = load_cell_data(0, GroupOutput)?;
    // check cluster proxy in Deps
    let proxy_index = find_position_by_type_hash(proxy_type_hash.as_slice(), CellDep)
        .ok_or(Error::ProxyCellNotInDep)?;
    let proxy_type = load_cell_type(proxy_index, CellDep)?.unwrap_or_default();
    if !is_valid_cluster_proxy_cell(&proxy_type.code_hash().as_slice().try_into().unwrap()) {
        return Err(Error::RefCellNotClusterProxy);
    }

    // verify cluster id
    let cluster_id = load_cell_data(proxy_index, CellDep)?;
    let script = load_script()?;
    if script.args().raw_data().as_ref() != &cluster_id {
        return Err(Error::InvalidAgentArgs);
    }

    // Condition 1: Check if cluster proxy exist in Inputs & Outputs
    let proxy_cell_in_input =
        find_position_by_type_hash(proxy_type_hash.as_slice(), Input).is_some();
    let proxy_cell_in_output =
        find_position_by_type_hash(proxy_type_hash.as_slice(), Output).is_some();

    if !proxy_cell_in_input || !proxy_cell_in_output {
        // Condition 2: Check for minimal payment
        let proxy_type_args = load_cell_type(proxy_index, CellDep)?
            .unwrap_or_default()
            .args()
            .raw_data();
        if proxy_type_args.len() == CLUSTER_PROXY_ID_WITH_PAYMENT_LEN {
            let range = CLUSTER_PROXY_ID_LEN..CLUSTER_PROXY_ID_WITH_PAYMENT_LEN;
            let minimal_payment =
                u64::from_le_bytes(proxy_type_args[range].try_into().unwrap_or_default());
            debug!("Minimal payment is: {}", minimal_payment);

            let proxy_lock_hash = load_cell_lock_hash(proxy_index, CellDep)?;
            let input_capacity = calc_capacity_sum(&proxy_lock_hash, Input);
            let output_capacity = calc_capacity_sum(&proxy_lock_hash, Output);
            if input_capacity + minimal_payment > output_capacity {
                return Err(Error::PaymentNotEnough);
            } else {
                // Condition 3: Check no same agent in creation
                if has_conflict_agent(Source::Output, &proxy_type_hash) {
                    return Err(Error::ConflictAgentCells);
                }
            }
        } else {
            if proxy_type_args.len() != CLUSTER_PROXY_ID_LEN {
                return Err(Error::PaymentMethodNotSupport);
            }
        }
    }

    // co-build check @lyk
    let action::SporeActionUnion::MintAgent(mint) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if &cluster_id != mint.cluster_id().as_slice()
        || &proxy_type.args().raw_data()[..CLUSTER_PROXY_ID_LEN] != mint.proxy_id().as_slice()
    {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupOutput, mint.to())?;

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    let input_agent_data = load_cell_data(0, GroupInput)?;
    let output_agent_data = load_cell_data(0, GroupOutput)?;

    if input_agent_data != output_agent_data || input_agent_data.is_empty() {
        return Err(Error::ImmutableAgentFieldModification);
    }

    // co-build check @lyk
    let action::SporeActionUnion::TransferAgent(transfer) = extract_spore_action()?.to_enum()
    else {
        return Err(Error::SporeActionMismatch);
    };
    if &load_self_id()? != transfer.cluster_id().as_slice() {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, transfer.from())?;
    check_spore_address(GroupOutput, transfer.to())?;

    Ok(())
}

fn process_destruction() -> Result<(), Error> {
    // co-build check @lyk
    let action::SporeActionUnion::BurnAgent(burn) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if &load_self_id()? != burn.cluster_id().as_slice() {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, burn.from())?;
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let agent_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if agent_in_output.len() > 1 {
        // Conflict Creation/Multiplier
        return Err(Error::InvalidAgentOperation);
    }

    let agent_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if agent_in_input.len() > 1 {
        // Multi-spend
        return Err(Error::InvalidAgentOperation);
    }

    return match (agent_in_input.len(), agent_in_output.len()) {
        (0, 1) => {
            // Creation
            let output_index =
                find_position_by_type(&agent_in_output[0], Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
            return process_creation(output_index);
        }
        (1, 0) => process_destruction(),
        (1, 1) => process_transfer(),
        _ => unreachable!(),
    };
}
