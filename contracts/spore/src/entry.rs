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
use ckb_std::high_level::load_transaction;
use ckb_std::syscalls::load_header_by_field;

use spore_types::generated::spore_types::SporeData;
use spore_utils::{find_position_by_type, MIME, verify_type_id};

use crate::error::Error;
use crate::error::Error::{ConflictCreation, MultipleSpend};

pub const CLUSTER_CODE_HASHES: [[u8; 32]; 1] = [
    [
        89, 141, 121, 61, 239, 239, 54, 226,
        238, 186, 84, 169, 180, 81, 48, 228,
        202, 146, 130, 46, 29, 25, 54, 113,
        244, 144, 149, 12, 59, 133, 96, 128
    ]
];


fn load_nft_data(index: usize, source: Source) -> Result<SporeData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let nft_data =
        SporeData::from_compatible_slice(raw_data.as_slice()).map_err(|_| Error::InvalidNFTData)?;
    Ok(nft_data)
}

fn get_position_by_type_args(args: &[u8], source: Source) -> Option<usize> {
    QueryIter::new(load_cell_type, source).position(|x| {
        match x {
            Some(script) => {
                CLUSTER_CODE_HASHES.contains(&script.code_hash().unpack())
                    && script.args().as_slice()[..] == args[..]
            }
            _ => false,
        }
    })
}

fn process_creation(index: usize) -> Result<(), Error> {
    let nft_data = load_nft_data(index, Output)?;

    if nft_data.content().is_empty() {
        return Err(Error::EmptyContent);
    }

    if nft_data.content_type().is_empty() {
        return Err(Error::InvalidContentType);
    }

    // verify NFT ID
    if !verify_type_id(index, Output) {
        return Err(Error::InvalidNFTID);
    }

    let _ = MIME::parse(nft_data.content_type()).map_err(|_| Error::InvalidContentType)?; // content_type validation

    if nft_data.cluster_id().to_opt().is_some() {
        // need to check if group cell in deps
        let group_id = nft_data.cluster_id().to_opt().unwrap_or_default();
        let group_id = group_id.as_slice();
        get_position_by_type_args(&group_id, CellDep).ok_or(Error::ClusterCellNotInDep)?;
        get_position_by_type_args(&group_id, Input)
            .ok_or(Error::ClusterCellCanNotUnlock)?;
        get_position_by_type_args(&group_id, Output)
            .ok_or(Error::ClusterCellCanNotUnlock)?;
    }

    Ok(())
}

fn process_destruction() -> Result<(), Error> {
    //destruction
    let nft_data = load_nft_data(0, GroupInput)?;

    let mime = MIME::parse(nft_data.content_type()).map_err(|_| Error::InvalidContentType)?;

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
