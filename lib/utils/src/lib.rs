#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::{prelude::*};
use ckb_std::ckb_types::util::hash::Blake2bBuilder;
use ckb_std::high_level::{load_cell_type, load_input};

pub use mime::MIME;

mod mime;

pub fn verify_type_id(index: usize, source: Source) -> bool {
    let first_input = match load_input(0, Source::Input) {
        Ok(cell_input) => cell_input,
        Err(_) => return false,
    };

    let verify_id = calc_type_id(first_input.as_slice(), index);
    let script_args: Vec<u8> = load_cell_type(index, source).unwrap_or(None).unwrap_or_default().args().unpack();
    let type_id = script_args.as_slice();
    type_id[..] == verify_id[..]
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

pub fn type_hash_filter_builder(
    type_hash: [u8; 32]
) -> impl Fn(&Option<[u8; 32]>) -> bool {
    move |script_hash: &Option<[u8; 32]>| match script_hash {
        Some(script_hash) => {
            script_hash[..] == type_hash[..]
        }
        _ => false,
    }
}
