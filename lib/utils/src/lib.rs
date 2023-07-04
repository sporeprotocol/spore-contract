#![no_std]

extern crate alloc;
mod mime;

use core::result::Result;
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::prelude::*;
use ckb_std::ckb_types::util::hash::Blake2bBuilder;
use ckb_std::high_level::{load_cell_type, load_input};
pub use mime::MIME;

pub fn verify_type_id(index: usize, source: Source) -> bool {
    let first_input = match load_input(0, Source::Input) {
        Ok(cell_input) => cell_input,
        Err(_) => return false,
    };
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(first_input.as_slice());
    blake2b.update(&(index as u64).to_le_bytes());
    let mut verify_id = [0; 32];
    blake2b.finalize(&mut verify_id);

    let nft_id: ckb_std::ckb_types::bytes::Bytes = match load_cell_type(index, source) {
        Ok(script) => script.unwrap_or_default().args().unpack(),
        Err(_) => return false,
    };
    nft_id[..] == verify_id[..]
}