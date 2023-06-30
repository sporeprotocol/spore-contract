// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::{vec, vec::Vec};

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    debug,
    high_level::{load_input, load_script, load_tx_hash},
    ckb_types::{bytes::Bytes, prelude::*},
};
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::packed::Script;
use ckb_std::ckb_types::util::hash::Blake2bBuilder;
use ckb_std::high_level::load_cell_data;


use crate::error::Error;

use cellular_utils::{ count_cells_by_type, load_index_by_type};

use cellular_types::generated::cellular_types::CellularSeriesData;

enum Action {
    Creation,
}

fn parse_series_action(series_type: &Script) -> Result<Action, Error> {
    let count_cells = |source| {
        count_cells_by_type(source, &|type_: &Script| {
            type_.as_slice() == series_type.as_slice()
        })
    };
    let series_cells_count = (count_cells(Source::Input), count_cells(Source::Output));
    match series_cells_count {
        (0, 1) => Ok(Action::Creation),
        _ => Err(Error::SeriesCellCountError),
    }
}


pub fn main() -> Result<(), Error> {
    let script = load_script()?;

    let first_input = load_input(0, Source::Input)?;
    let first_output_index = load_index_by_type(Source::Output, &script);
    if first_output_index.is_none() {
        return Err(Error::SeriesCellCountError);
    }
    let first_output_index = first_output_index.unwrap();
    let mut blake2b = Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(first_input.as_slice());
    blake2b.update(&(first_output_index as u64).to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);

    let series_args: Bytes = script.args().unpack();

    if series_args[..] != ret[..] {
        return Err(Error::InvalidTypesArg);
    }

    let cell_data = load_cell_data(first_output_index, Source::Output)?;

    let series_data = CellularSeriesData::from_slice(cell_data.as_slice());

    if series_data.is_err() {
        return Err(Error::InvalidSeriesData);
    }

    let series_data = series_data.unwrap();

    if series_data.name().is_empty() {
        return Err(Error::EmptyName);
    }

    Ok(())
}
