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
use ckb_std::{ckb_types::prelude::*, high_level::load_script};

use spore_errors::error::Error;
use spore_utils::{calc_capacity_sum, find_position_by_type, find_position_by_type_hash};
use spore_utils::{CLUSTER_PROXY_ID_LEN, CLUSTER_PROXY_ID_WITH_PAYMENT_LEN};

fn is_valid_cluster_proxy_cell(script: &Script) -> bool {
    spore_utils::matches_code_identity(
        script,
        &crate::hash::CLUSTER_PROXY_CODE_HASHES,
        &crate::hash::CLUSTER_PROXY_TYPE_HASHES,
    )
}

fn is_agent_identity(script: &Script, active: &Script, historical: &[[u8; 32]]) -> bool {
    (script.code_hash().as_slice() == active.code_hash().as_slice()
        && script.hash_type() == active.hash_type())
        || spore_utils::matches_code_identity(script, historical, &[])
}

fn consumes_and_recreates_proxy(proxy_hash: &[u8]) -> bool {
    find_position_by_type_hash(proxy_hash, Input).is_some()
        && find_position_by_type_hash(proxy_hash, Output).is_some()
}

fn minimum_payment(proxy_type: &Script) -> Result<Option<u64>, Error> {
    let args = proxy_type.args().raw_data();
    match args.len() {
        CLUSTER_PROXY_ID_LEN => Ok(None),
        CLUSTER_PROXY_ID_WITH_PAYMENT_LEN => Ok(Some(u64::from_le_bytes(
            args[CLUSTER_PROXY_ID_LEN..].try_into().unwrap(),
        ))),
        _ => Err(Error::PaymentMethodNotSupport),
    }
}

fn total_agent_payment(
    output_index: usize,
    active: &Script,
    proxy_hash: &[u8],
    beneficiary: &[u8; 32],
    own_payment: u64,
) -> Result<u64, Error> {
    let mut total = own_payment;
    for (index, script) in QueryIter::new(load_cell_type, Output).enumerate() {
        let Some(script) = script else { continue };
        if index == output_index
            || !is_agent_identity(&script, active, &crate::hash::HISTORICAL_AGENT_CODE_HASHES)
        {
            continue;
        }
        let other_hash = load_cell_data(index, Output)?;
        // Preserve the existing one-output rule for agents of one paid Proxy.
        if other_hash == proxy_hash {
            return Err(Error::ConflictAgentCells);
        }
        // Transfers and creation via direct Proxy ownership owe no payment.
        if other_hash.len() != 32
            || find_position_by_type(&script, Input).is_some()
            || consumes_and_recreates_proxy(&other_hash)
        {
            continue;
        }
        let Some(proxy_index) = find_position_by_type_hash(&other_hash, CellDep) else {
            continue;
        };
        let Some(proxy) = load_cell_type(proxy_index, CellDep)? else {
            continue;
        };
        if !is_valid_cluster_proxy_cell(&proxy)
            || load_cell_lock_hash(proxy_index, CellDep)? != *beneficiary
            || load_cell_data(proxy_index, CellDep)? != script.args().raw_data().as_ref()
        {
            continue;
        }
        if let Some(payment) = minimum_payment(&proxy)? {
            total = total.checked_add(payment).ok_or(Error::PaymentNotEnough)?;
        }
    }
    Ok(total)
}

// Agent data is the proxy's full type-script hash, not an intent field.
fn load_proxy_hash(source: Source) -> Result<Vec<u8>, Error> {
    let data = load_cell_data(0, source)?;
    if data.len() != 32 {
        return Err(Error::InvalidAgentData);
    }
    Ok(data)
}

fn process_creation(index: usize) -> Result<(), Error> {
    let proxy_type_hash = load_proxy_hash(GroupOutput)?;
    // check cluster proxy in Deps
    let proxy_index = find_position_by_type_hash(proxy_type_hash.as_slice(), CellDep)
        .ok_or(Error::ProxyCellNotInDep)?;
    let proxy_type = load_cell_type(proxy_index, CellDep)?.unwrap_or_default();
    if !is_valid_cluster_proxy_cell(&proxy_type) {
        return Err(Error::RefCellNotClusterProxy);
    }

    // verify cluster id
    let cluster_id = load_cell_data(proxy_index, CellDep)?;
    if cluster_id.len() != 32 {
        return Err(Error::InvalidAgentData);
    }
    let script = load_script()?;
    if script.args().raw_data().as_ref() != &cluster_id {
        return Err(Error::InvalidAgentArgs);
    }

    if !consumes_and_recreates_proxy(&proxy_type_hash) {
        if let Some(payment) = minimum_payment(&proxy_type)? {
            let beneficiary = load_cell_lock_hash(proxy_index, CellDep)?;
            let paid = calc_capacity_sum(&beneficiary, Output)
                .checked_sub(calc_capacity_sum(&beneficiary, Input))
                .filter(|paid| *paid >= payment)
                .ok_or(Error::PaymentNotEnough)?;
            let required =
                total_agent_payment(index, &script, &proxy_type_hash, &beneficiary, payment)?;
            if paid < required {
                return Err(Error::PaymentNotEnough);
            }
        }
    }

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    let input_agent_data = load_proxy_hash(GroupInput)?;
    let output_agent_data = load_proxy_hash(GroupOutput)?;

    if input_agent_data != output_agent_data || input_agent_data.is_empty() {
        return Err(Error::ImmutableAgentFieldModification);
    }

    Ok(())
}

fn process_destruction() -> Result<(), Error> {
    load_proxy_hash(GroupInput)?;
    Ok(())
}

pub fn main() -> Result<(), Error> {
    if load_script()?.args().raw_data().len() != 32 {
        return Err(Error::InvalidAgentArgs);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ckb_std::ckb_types::packed::Byte32;

    #[test]
    fn conflict_check_recognizes_historical_agent_identities() {
        let active = Script::new_builder()
            .code_hash(Byte32::from_slice(&[1; 32]).unwrap())
            .hash_type(1u8.into())
            .build();
        let legacy_hash = [2; 32];
        let legacy = Script::new_builder()
            .code_hash(Byte32::from_slice(&legacy_hash).unwrap())
            .hash_type(2u8.into())
            .build();
        assert!(is_agent_identity(&active, &active, &[legacy_hash]));
        assert!(is_agent_identity(&legacy, &active, &[legacy_hash]));
        assert!(!is_agent_identity(&legacy, &active, &[]));
        let different_type = legacy.as_builder().hash_type(1u8.into()).build();
        assert!(!is_agent_identity(&different_type, &active, &[legacy_hash]));
    }
}
