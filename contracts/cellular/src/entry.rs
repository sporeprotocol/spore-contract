// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::{vec, vec::Vec};
use alloc::collections::BTreeMap;
use core::iter::Map;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::Script, prelude::*},
    debug,
    high_level::{load_cell_data, load_script, load_tx_hash, load_cell_type, QueryIter},
};
use ckb_std::high_level::{load_cell_capacity, load_cell_lock, load_cell_type_hash};

use cellular_types::generated::cellular_types::CellularData;

use cellular_utils::{count_cells_by_type, load_index_by_type};

use crate::error::Error;

pub enum CellularAction {
    Creation,
    Destruction,
    Update, // Can be transfer or other extensional updates
}


fn get_cellulars_data(source: Source, cellular_type: &Script) -> Vec<(usize, Vec<u8>)> {
    QueryIter::new(load_cell_type, source)
        .enumerate()
        .filter(|(_, cell_type)| cell_type.clone().unwrap_or_default().code_hash().as_slice() == cellular_type.code_hash().as_slice())
        .map(|(index, _)| (index, load_cell_data(index, source).map_or_else(|_| Vec::new(), |data| data))).collect()
}

fn get_cellulars_script(source: Source, cellular_type: &Script) -> Vec<(usize, Script)> {
    QueryIter::new(load_cell_type, source)
        .enumerate()
        .filter(|(_, cell_type)| cell_type.clone().unwrap_or_default().code_hash().as_slice() == cellular_type.code_hash().as_slice())
        .map(|(index, script)| (index, script.unwrap_or_default())).collect()
}

fn match_cellular_action(cellular_type: &Script) -> Result<CellularAction, Error> {
    let count_cells = |source| {
        count_cells_by_type(source, &|type_: &Script| {
            type_.as_slice() == cellular_type.as_slice()
        })
    };

    let (input_count, output_count) = (count_cells(Source::Input), count_cells(Source::Output));

    if input_count == 0 && output_count > 0 {
        Ok(CellularAction::Creation)
    } else if input_count > output_count {
        Ok(CellularAction::Destruction)
    } else if input_count == output_count {
        Ok(CellularAction::Update)
    } else {
        Err(Error::ConflictDualOperation)
    }
}

fn handle_creation(cellular_type: &Script) -> Result<(), Error> {
    // do capacity check
    let input_capacity = QueryIter::new(load_cell_capacity, Source::Input)
        .map(|capacity| capacity).sum::<u64>();
    let output_capacity = QueryIter::new(load_cell_capacity, Source::Output)
        .map(|capacity| capacity).sum::<u64>();

    if input_capacity < output_capacity {
        return Err(Error::InsufficientCapacity);
    }


    get_cellulars_data(Source::Output, cellular_type).into_iter().try_for_each(|(_, raw_data)|{
        let cellular_data = CellularData::from_slice(raw_data.as_slice());
        if cellular_data.is_err() {
            return Err(Error::InvalidCellularData);
        }
        let cellular_data = cellular_data.unwrap_or_default();
        if cellular_data.content_type().is_empty() { // content cannot be empty while creation
            return Err(Error::EmptyContent)
        }

        // validate series cell in dep is series set
        if cellular_data.series().is_some() {
            let series = cellular_data.series().to_opt().unwrap();
            let series_pos = QueryIter::new(load_cell_type,Source::CellDep)
                .position(|type_script| type_script.map_or(false, |type_script| type_script.args().as_slice() == series.as_slice()));

            if series_pos.is_none() {
                return Err(Error::SeriesNotInDep)
            }
        }

        Ok(())
    })?;

    Ok(())
}

fn handle_destruction(cellular_type: &Script) -> Result<(), Error> {
    let input_cell_data = get_cellulars_data(Source::Input, cellular_type);

    let output_cell_data = get_cellulars_data(Source::Output, cellular_type);

    if !output_cell_data.is_empty() {
        return Err(Error::ConflictDualOperation); // can not do creation/update/destruction at a same time
    }

    input_cell_data.into_iter().try_for_each(|(index, raw_data)| {
        let cellular_data = CellularData::from_slice(raw_data.as_slice());
        if cellular_data.is_err() {
            return Err(Error::InvalidCellularData);
        }
        let cellular_data = cellular_data.unwrap_or_default();
        if bool::from(cellular_data.immortal()) { // try to destroy a immortal cellular
            return Err(Error::DestroyImmortalCellular)
        } else {
            Ok(())
        }
    })?;
    Ok(())
}

fn handle_update(cellular_type: &Script) -> Result<(), Error> {

    let mut input_ids: Vec<Vec<u8>> = Vec::new();
    let input_scripts = get_cellulars_script(Source::Input, cellular_type);
    let output_scripts = get_cellulars_script(Source::Output, cellular_type);
    input_scripts.into_iter().for_each(|(index, script)| {
        let cnft_id = script.args().as_slice().to_vec();
        input_ids.push(cnft_id);
    });

    output_scripts.into_iter().for_each(|(_, script)| {
        let cnft_id = script.args().as_slice().to_vec();
        if input_ids.contains(&cnft_id) {
            input_ids.retain(|x| x != &cnft_id)
        }
    });

    if !input_ids.is_empty() {
        return Err(Error::InvalidUpdate);
    }


    let input_lock = load_cell_lock(0, Source::GroupInput)?;
    let output_lock = load_cell_lock(0, Source::GroupOutput)?;
    if input_lock.as_slice() != output_lock.as_slice() {
        return Err(Error::LockedNFT);
    }
    Ok(())
}


pub fn main() -> Result<(), Error> {
    let cellular_type = load_script()?;

    match match_cellular_action(&cellular_type)? {
        CellularAction::Creation => { handle_creation(&cellular_type) },
        CellularAction::Destruction => { handle_destruction(&cellular_type) },
        CellularAction::Update => { handle_update(&cellular_type) },
    }
}

