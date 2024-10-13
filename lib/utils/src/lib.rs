#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use blake2b_ref::Blake2bBuilder;
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::bytes::Bytes;
use ckb_std::ckb_types::packed::Script;
use ckb_std::ckb_types::prelude::*;
use ckb_std::debug;
use ckb_std::high_level::{
    load_cell, load_cell_data, load_cell_lock, load_cell_lock_hash, load_cell_type,
    load_cell_type_hash, load_input, load_script, load_script_hash, QueryIter,
};

use spore_errors::error::Error;
use spore_types::generated::{action, spore};

pub use mime::MIME;
pub mod co_build_types {
    pub use ckb_transaction_cobuild::schemas::basic::*;
    pub use ckb_transaction_cobuild::schemas::top_level::*;
}

mod mime;

pub const MUTANT_ID_LEN: usize = 32;
pub const MUTANT_ID_WITH_PAYMENT_LEN: usize = MUTANT_ID_LEN + 8;

pub const CLUSTER_PROXY_ID_LEN: usize = 32;
pub const CLUSTER_PROXY_ID_WITH_PAYMENT_LEN: usize = CLUSTER_PROXY_ID_LEN + 8;

pub fn load_self_id() -> Result<Vec<u8>, Error> {
    Ok(load_script()?.args().raw_data()[..32].to_vec())
}

pub fn load_type_args(index: usize, source: Source) -> Bytes {
    load_cell_type(index, source)
        .unwrap_or(None)
        .unwrap_or_default()
        .args()
        .raw_data()
}

// only be avaliable in mint/create like ckb transaction
pub fn verify_type_id(output_index: usize) -> Option<[u8; 32]> {
    let first_input = match load_input(0, Source::Input) {
        Ok(cell_input) => cell_input,
        Err(_) => return None,
    };

    let expected_id = calc_type_id(first_input.as_slice(), output_index);
    let type_id_args = load_type_args(output_index, Source::Output);

    debug!("wanted: {expected_id:?}");
    debug!("got({output_index}): {type_id_args:?}");
    if type_id_args.len() < 32 {
        return None;
    }
    if type_id_args.as_ref()[..32] == expected_id {
        return Some(expected_id);
    }

    None
}

/// The type ID is calculated as the blake2b (with CKB's personalization) of
/// the first CellInput in current transaction, and the created output cell
/// index(in 64-bit little endian unsigned integer).
pub fn calc_type_id(tx_first_input: &[u8], output_index: usize) -> [u8; 32] {
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(tx_first_input);
    blake2b.update(&(output_index as u64).to_le_bytes());
    let mut verify_id = [0; 32];
    blake2b.finalize(&mut verify_id);
    verify_id
}

pub fn type_hash_filter_builder(type_hash: [u8; 32]) -> impl Fn(&Option<[u8; 32]>) -> bool {
    move |script_hash: &Option<[u8; 32]>| match script_hash {
        Some(script_hash) => script_hash[..] == type_hash[..],
        _ => false,
    }
}

pub fn find_position_by_type_args(
    args: &[u8],
    source: Source,
    filter_fn: Option<fn(&[u8; 32]) -> bool>,
) -> Option<usize> {
    QueryIter::new(load_cell_type, source).position(|script| {
        if let Some(script) = script {
            if let Some(filter) = filter_fn {
                if !filter(&script.code_hash().as_slice().try_into().unwrap()) {
                    return false;
                }
            }
            script.args().raw_data().as_ref() == args
        } else {
            false
        }
    })
}

pub fn find_position_by_type(type_script: &Script, source: Source) -> Option<usize> {
    QueryIter::new(load_cell_type, source).position(|script| match script {
        Some(script) => script.as_bytes() == type_script.as_bytes(),
        _ => false,
    })
}

pub fn find_position_by_type_hash(type_hash: &[u8], source: Source) -> Option<usize> {
    QueryIter::new(load_cell_type_hash, source).position(|cell_type_hash| match cell_type_hash {
        None => false,
        Some(cell_type_hash) => {
            debug!(
                "cell_type_hash : {:?}, wanted: {:?}",
                cell_type_hash, type_hash
            );
            cell_type_hash[..] == type_hash[..]
        }
    })
}

pub fn find_position_by_type_and_data(
    target_data: &[u8],
    source: Source,
    filter_fn: Option<fn(&[u8; 32]) -> bool>,
) -> Option<usize> {
    QueryIter::new(load_cell_data, source)
        .enumerate()
        .position(|(index, data)| {
            data[..] == target_data[..]
                && match filter_fn {
                    None => true,
                    Some(ref filter_fn) => {
                        if let Some(type_hash) =
                            load_cell_type_hash(index, source).unwrap_or_default()
                        {
                            filter_fn(&type_hash)
                        } else {
                            false
                        }
                    }
                }
        })
}

pub fn find_position_by_lock_hash(lock_hash: &[u8; 32], source: Source) -> Option<usize> {
    QueryIter::new(load_cell_lock_hash, source).position(|hash| hash[..] == lock_hash[..])
}

pub fn calc_capacity_sum(lock_hash: &[u8; 32], source: Source) -> u64 {
    QueryIter::new(load_cell, source)
        .filter(|cell| cell.lock().calc_script_hash().raw_data().as_ref() == lock_hash)
        .map(|cell| ckb_std::ckb_types::prelude::Unpack::<u64>::unpack(&cell.capacity()))
        .sum()
}

pub fn check_spore_address(
    group_source: Source,
    spore_address: action::Address,
) -> Result<(), Error> {
    let address = load_cell_lock(0, group_source)?;
    let action::AddressUnion::Script(expected_script) = spore_address.to_enum();
    if address.as_slice() != expected_script.as_slice() {
        return Err(Error::SporeActionAddressesMismatch);
    }
    Ok(())
}

pub fn extract_spore_action() -> Result<action::SporeAction, Error> {
    let message = ckb_transaction_cobuild::fetch_message()
        .map_err(|_| Error::InvliadCoBuildWitnessLayout)?
        .ok_or(Error::InvliadCoBuildWitnessLayout)?;
    let script_hash = load_script_hash()?;

    let mut iter = message
        .actions()
        .into_iter()
        .filter(|value| value.script_hash().as_slice() == script_hash.as_slice());
    match (iter.next(), iter.next()) {
        (Some(action), None) => action::SporeAction::from_slice(&action.data().raw_data())
            .map_err(|_| Error::InvliadCoBuildMessage),
        _ => Err(Error::SporeActionDuplicated),
    }
}

pub fn compatible_load_cluster_data(
    raw_cluster_data: &[u8],
) -> Result<spore::ClusterDataV2, Error> {
    let cluster_data = spore::ClusterData::from_compatible_slice(raw_cluster_data)
        .map_err(|_| Error::InvalidClusterData)?;
    debug!("cluster_data filed count: {}", cluster_data.field_count());
    if cluster_data.field_count() == 2 {
        Ok(spore::ClusterDataV2::new_builder()
            .name(cluster_data.name())
            .description(cluster_data.description())
            .mutant_id(Default::default())
            .build())
    } else {
        Ok(
            spore::ClusterDataV2::from_compatible_slice(raw_cluster_data)
                .map_err(|_| Error::InvalidClusterData)?,
        )
    }
}

pub fn blake2b_256<T: AsRef<[u8]>>(s: T) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(s.as_ref());
    blake2b.finalize(&mut result);
    result
}
