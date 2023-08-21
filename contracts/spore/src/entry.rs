use alloc::{string::ToString, vec::Vec};
use core::result::Result;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    high_level::{load_cell_data, load_cell_type, QueryIter},
};
use ckb_std::ckb_constants::HeaderField;
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, HeaderDep, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell_lock_hash, load_transaction};
use ckb_std::syscalls::load_header_by_field;

use spore_types::generated::spore_types::SporeData;
use spore_utils::{find_position_by_type, find_position_by_lock, find_position_by_type_arg, MIME, verify_type_id};
use spore_constant::CLUSTER_CODE_HASHES;

use crate::error::Error;
use crate::error::Error::{ConflictCreation, MultipleSpend};


fn load_spore_data(index: usize, source: Source) -> Result<SporeData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let spore_data =
        SporeData::from_compatible_slice(raw_data.as_slice()).map_err(|_| Error::InvalidNFTData)?;
    Ok(spore_data)
}

fn process_creation(index: usize) -> Result<(), Error> {
    let spore_data = load_spore_data(index, Output)?;

    if spore_data.content().is_empty() {
        return Err(Error::EmptyContent);
    }

    if spore_data.content_type().is_empty() {
        return Err(Error::InvalidContentType);
    }

    // verify NFT ID
    if !verify_type_id(index, Output) {
        return Err(Error::InvalidNFTID);
    }

    let _ = MIME::parse(spore_data.content_type()).map_err(|_| Error::InvalidContentType)?; // content_type validation

    if spore_data.cluster_id().to_opt().is_some() {
        // check if cluster cell in deps
        let cluster_id = spore_data.cluster_id().to_opt().unwrap_or_default();
        let cluster_id = cluster_id.as_slice();
        let filter_fn: fn(&[u8; 32]) -> bool = |x| -> bool { CLUSTER_CODE_HASHES.contains(x) };
        let cell_dep_index = find_position_by_type_arg(&cluster_id, CellDep, Some(filter_fn)).ok_or(Error::ClusterCellNotInDep)?;

        // Condition 1: Check if cluster exist in Inputs & Outputs
        return if find_position_by_type_arg(&cluster_id, Input, Some(filter_fn)).is_some()
            && find_position_by_type_arg(&cluster_id, Output, Some(filter_fn)).is_some() {
            Ok(())
        } else {
            // Condition 2: Check if Lock Proxy exist in Inputs & Outputs
            let cluster_lock_hash = load_cell_lock_hash(cell_dep_index, CellDep)?;
            find_position_by_lock(&cluster_lock_hash, Output).ok_or(Error::ClusterOwnershipVerifyFailed)?;
            find_position_by_lock(&cluster_lock_hash, Input).ok_or(Error::ClusterOwnershipVerifyFailed)?;
            Ok(())
        }
    }

    Ok(())
}



fn process_destruction() -> Result<(), Error> {
    //destruction
    let spore_data = load_spore_data(0, GroupInput)?;

    let mime = MIME::parse(spore_data.content_type()).map_err(|_| Error::InvalidContentType)?;

    let immortal = if mime.params().contains_key("immortal") {
        mime.params()
            .get("immortal")
            .unwrap_or(&"".to_string())
            .trim()
            .to_ascii_lowercase()
            == "true"
    } else {
        false
    };

    if immortal {
        // true destroy a immortal nft
        return Err(Error::DestroyImmortalNFT);
    }

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    // found same NFT in output, this is a transfer
    // check no field was modified
    let input_nft_data = load_cell_data(0, GroupInput)?;
    let output_nft_data = load_cell_data(0, GroupOutput)?;

    if input_nft_data[..] != output_nft_data[..] {
        return Err(Error::ModifyPermanentField);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let spore_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| {
            script.unwrap_or_default()
        }).collect();

    if spore_in_output.len() > 1 {
        return Err(ConflictCreation);
    }

    let spore_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| {
            script.unwrap_or_default()
        }).collect();

    if spore_in_input.len() > 1 {
        return Err(MultipleSpend);
    }

    match (spore_in_input.len(), spore_in_output.len()) {
        (0, 1) => {
            // find it's index in Source::Output
            let output_index = find_position_by_type(spore_in_output[0].as_slice(), Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
            return process_creation(output_index);
        }
        (1, 0) => { return process_destruction(); }
        (1, 1) => { return process_transfer(); }
        _ => unreachable!(),
    }
}
