#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::packed::Script;
use ckb_std::ckb_types::prelude::*;
use ckb_std::ckb_types::util::hash::Blake2bBuilder;
use ckb_std::debug;
use ckb_std::high_level::{
    load_cell, load_cell_data, load_cell_lock_hash, load_cell_type, load_cell_type_hash,
    load_input, QueryIter,
};

pub use mime::MIME;

mod mime;

pub fn verify_type_id(index: usize, source: Source) -> bool {
    let first_input = match load_input(0, Source::Input) {
        Ok(cell_input) => cell_input,
        Err(_) => return false,
    };

    let verify_id = calc_type_id(first_input.as_slice(), index);
    let script_args: Vec<u8> = load_cell_type(index, source)
        .unwrap_or(None)
        .unwrap_or_default()
        .args()
        .unpack();
    let type_id = script_args.as_slice();
    debug!("wanted: {:?}, got: {:?}", verify_id, type_id);
    type_id[..32] == verify_id[..]
}

pub fn calc_type_id(prevout_hash: &[u8], output_index: usize) -> [u8; 32] {
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(prevout_hash);
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
            script.args().raw_data().as_ref() == args
                && match &filter_fn {
                    None => true,
                    Some(ref filter_fn) => filter_fn(&script.code_hash().unpack()),
                }
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

pub fn find_posityion_by_type_hash(type_hash: &[u8], source: Source) -> Option<usize> {
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

pub fn calc_capacity_sum(lock_hash: &[u8; 32], source: Source) -> u128 {
    QueryIter::new(load_cell, source)
        .filter(|cell| cell.lock().calc_script_hash().raw_data().as_ref() == lock_hash)
        .map(|cell| cell.capacity().unpack() as u128)
        .sum()
}
