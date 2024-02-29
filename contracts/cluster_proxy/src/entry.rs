use alloc::vec::Vec;
use ckb_std::ckb_types::prelude::Entity;
use core::result::Result;

// Import from `core` instead of from `std` since we are in no-std mode
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{
    load_cell_data, load_cell_lock_hash, load_cell_type, load_script, QueryIter,
};

use spore_errors::error::Error;
use spore_types::generated::action;
use spore_utils::{
    check_spore_address, extract_spore_action, find_position_by_lock_hash, find_position_by_type,
    find_position_by_type_args, load_self_id, verify_type_id, CLUSTER_PROXY_ID_LEN,
    CLUSTER_PROXY_ID_WITH_PAYMENT_LEN,
};

fn is_valid_cluster_cell(script_hash: &[u8; 32]) -> bool {
    crate::hash::CLUSTER_CODE_HASHES.contains(script_hash)
}

fn process_creation(index: usize) -> Result<(), Error> {
    let cluster_id = load_cell_data(0, GroupOutput)?;
    // check cluster in Deps
    let cell_dep_index =
        find_position_by_type_args(&cluster_id, CellDep, Some(is_valid_cluster_cell))
            .ok_or(Error::ClusterCellNotInDep)?;

    // verify script args format
    let args = load_script()?.args().raw_data();
    if args.len() != CLUSTER_PROXY_ID_LEN && args.len() != CLUSTER_PROXY_ID_WITH_PAYMENT_LEN {
        return Err(Error::InvalidProxyArgs);
    }

    // verify Proxy ID
    let Some(proxy_id) = verify_type_id(index) else {
        return Err(Error::InvalidProxyID);
    };

    // Condition 1: Check if cluster exist in Inputs & Outputs
    let cluster_cell_in_input =
        find_position_by_type_args(&cluster_id, Input, Some(is_valid_cluster_cell)).is_some();
    let cluster_cell_in_output =
        find_position_by_type_args(&cluster_id, Output, Some(is_valid_cluster_cell)).is_some();

    if !cluster_cell_in_input || !cluster_cell_in_output {
        // Condition 2: Check if Lock Proxy exist in Inputs & Outputs
        let cluster_lock_hash = load_cell_lock_hash(cell_dep_index, CellDep)?;
        find_position_by_lock_hash(&cluster_lock_hash, Output)
            .ok_or(Error::ClusterOwnershipVerifyFailed)?;
        find_position_by_lock_hash(&cluster_lock_hash, Input)
            .ok_or(Error::ClusterOwnershipVerifyFailed)?;
    }

    // co-build check @lyk
    let action::SporeActionUnion::MintProxy(create) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if proxy_id != create.proxy_id().as_slice() || &cluster_id != create.cluster_id().as_slice() {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupOutput, create.to())?;

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    let input_data = load_cell_data(0, GroupInput)?;
    let output_data = load_cell_data(0, GroupOutput)?;

    if input_data != output_data {
        return Err(Error::ImmutableProxyFieldModification);
    }

    // co-build check @lyk
    let action::SporeActionUnion::TransferProxy(transfer) = extract_spore_action()?.to_enum()
    else {
        return Err(Error::SporeActionMismatch);
    };
    if input_data.as_slice() != transfer.cluster_id().as_slice()
        || &load_self_id()? != transfer.proxy_id().as_slice()
    {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, transfer.from())?;
    check_spore_address(GroupOutput, transfer.to())?;

    Ok(())
}

fn process_destruction() -> Result<(), Error> {
    let cluster_id = load_cell_data(0, GroupInput)?;
    let proxy_id = load_self_id()?;

    // co-build check @lyk
    let action::SporeActionUnion::BurnProxy(burn) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if cluster_id.as_slice() != burn.cluster_id().as_slice()
        || &proxy_id != burn.proxy_id().as_slice()
    {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, burn.from())?;
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let proxy_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if proxy_in_output.len() > 1 {
        // Conflict Creation/Multiplier
        return Err(Error::InvalidProxyOperation);
    }

    let proxy_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if proxy_in_input.len() > 1 {
        // Multi-spend
        return Err(Error::InvalidProxyOperation);
    }

    return match (proxy_in_input.len(), proxy_in_output.len()) {
        (0, 1) => {
            // Creation
            let output_index =
                find_position_by_type(&proxy_in_output[0], Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
            return process_creation(output_index);
        }
        (1, 0) => process_destruction(),
        (1, 1) => process_transfer(),
        _ => unreachable!(),
    };
}
