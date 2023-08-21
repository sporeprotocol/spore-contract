// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::{vec, vec::Vec};

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    debug,
    high_level::{load_script, load_tx_hash},
    ckb_types::{bytes::Bytes, prelude::*},
};
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell, load_cell_data, load_cell_lock_hash, load_cell_type, QueryIter};
use spore_utils::{find_position_by_type, verify_type_id};

use crate::error::Error;

const CLUSTER_PROXY_ID_LEN: usize = 32;

fn calc_capacity_sum(lock_hash: &[u8;32], source: Source) -> u128 {
    QueryIter::new(load_cell, source)
        .filter(|cell| {
            cell.lock().calc_script_hash().as_slice()[..] == lock_hash[..]
        })
        .map(|cell| {
            cell.capacity().unpack() as u128
        }).sum()
}


fn process_creation(index: usize) -> Result<(), Error> {

    let proxy_type_hash = load_cell_data(0, GroupOutput)?;

    // check cluster proxy in Deps
    let cell_dep_index = find_position_by_type(proxy_type_hash.as_slice(), CellDep).ok_or(Error::ProxyCellNotInDep)?;

    // verify Agent ID
    if !verify_type_id(index, Output) {
        return Err(Error::InvalidAgentID);
    }

    // Condition 1: Check if cluster proxy exist in Inputs & Outputs
    return if find_position_by_type(proxy_type_hash.as_slice(), Input).is_some()
        && find_position_by_type(proxy_type_hash.as_slice(), Output).is_some() {
        Ok(())
    } else {
        // Condition 2: Check for minimal payment
        let args = load_cell_type(cell_dep_index, CellDep)?.unwrap_or_default().args();
        if args.as_slice().len() > CLUSTER_PROXY_ID_LEN {
            let minimal_payment = 10u128.pow(args.as_slice()[CLUSTER_PROXY_ID_LEN] as u32);
            let lock = load_cell_lock_hash(cell_dep_index, CellDep)?;
            let input_capacity = calc_capacity_sum(&lock,Input);
            let output_capacity = calc_capacity_sum(&lock,Output);
            if input_capacity + minimal_payment > output_capacity {
                return Err(Error::PaymentNotEnough)
            }
        } else {
            return Err(Error::PaymentMethodNotSupport)
        }
        Ok(())
    }
}

fn process_transfer() -> Result<(), Error> {
    let input_agent_data = load_cell_data(0, GroupInput)?;
    let output_agent_data = load_cell_data(0, GroupOutput)?;

    if input_agent_data.as_slice()[..] != output_agent_data.as_slice()[..] {
        return Err(Error::ImmutableFieldModification)
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let agent_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| {
            script.unwrap_or_default()
        }).collect();

    if agent_in_output.len() > 1 {
        // Conflict Creation/Multiplier
        return Err(Error::InvalidOperation)
    }

    let agent_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| {
            script.unwrap_or_default()
        }).collect();

    if agent_in_input.len() > 1 {
        // Multi-spend
        return Err(Error::InvalidOperation);
    }

    return match (agent_in_input.len(), agent_in_output.len()) {
        (0, 1) => {
            // Creation
            let output_index = find_position_by_type(agent_in_output[0].as_slice(), Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
            return process_creation(output_index);
        },
        (1, 0) => Ok(()), // There's no limitation to destroy an agent except lock
        (1, 1) => process_transfer(),
        _ => unreachable!()
    };
}
