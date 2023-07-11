use alloc::{ string::ToString, vec::Vec };
use core::result::Result;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{prelude::*, util::hash::Blake2bBuilder},
    high_level::{load_cell_data, load_cell_type, load_cell_lock_hash, load_cell_type_hash, load_script_hash, QueryIter},
    error::SysError,
};
use ckb_std::ckb_types::core::ScriptHashType;

use cellular_types::generated::cellular_types::{Bytes, NFTData};

use crate::error::Error;
use cellular_utils::{MIME, verify_type_id};

fn load_nft_data(index: usize, source: Source) -> Result<NFTData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let nft_data = NFTData::from_slice(raw_data.as_slice()).map_err(|_| Error::InvalidNFTData)?;
    Ok(nft_data)
}

fn get_position_by_type_args(args: &Bytes, source: Source) -> Option<usize> {
    QueryIter::new(load_cell_type, source).position(|type_script| {
        type_script.map_or(false, |type_script| {
            type_script.args().as_slice()[..] == args.as_slice()[..]
        })
    })
}

fn process_input(
    index: usize,
    input_source: Source,
    cnft_in_outputs: &mut Vec<usize>,
    output_source: Source,
) -> Result<(), Error> {
    let cnft_id = load_cell_type(index, input_source)?
        .unwrap_or_default()
        .args();

    for i in 0..cnft_in_outputs.len() {
        let output_index = cnft_in_outputs.get(i).unwrap();
        let output_cnft_id = load_cell_type(*output_index, output_source)?
            .unwrap_or_default()
            .args();
        if cnft_id.as_slice()[..] == output_cnft_id.as_slice()[..] {
            // found same NFT in output, this is a transfer

            // check no field was modified

            let nft_data = load_nft_data(index, input_source)?;
            let output_nft_data = load_nft_data(i, output_source)?;

            if nft_data.content_type().as_slice()[..]
                != output_nft_data.content_type().as_slice()[..]
                || nft_data.content().as_slice()[..]
                    != output_nft_data.content_type().as_slice()[..]
                || nft_data.group().as_slice()[..] != output_nft_data.group().as_slice()[..]
            {
                return Err(Error::ModifyPermanentField);
            }

            cnft_in_outputs.remove(i);
            return Ok(());
        }
    }

    //destruction
    let nft_data = load_nft_data(index, input_source)?;

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

fn process_creation(index: usize, source: Source) -> Result<(), Error> {
    let nft_data = load_nft_data(index, source)?;

    if nft_data.content().is_empty() {
        return Err(Error::EmptyContent);
    }

    if nft_data.content_type().is_empty() {
        return Err(Error::InvalidContentType);
    }

    // verify NFT ID
    if !verify_type_id(index, source) {
        return Err(Error::InvalidNFTID);
    }


    MIME::parse(nft_data.content_type()).map_err(|_|Error::InvalidContentType)?; // content_type validation

    if nft_data.group().is_some() {
        // need to check if group cell in deps
        let group_id = nft_data.group().to_opt().unwrap();
        let group_cell_pos = get_position_by_type_args(&group_id, Source::CellDep);

        if group_cell_pos.is_none() {
            return Err(Error::GroupCellNotInDep);
        }

        let group_cell_pos = group_cell_pos.unwrap();

        // check ownership
        let lock_hash = load_cell_lock_hash(group_cell_pos, Source::CellDep)?;

        verify_group_cell(&lock_hash, group_id, Source::GroupInput)?;
    }

    Ok(())
}

fn verify_group_cell(lock_hash: &[u8; 32], group_id: Bytes, source: Source) -> Result<(), Error> {
    for i in 0.. {
        let cell_lock_hash = match load_cell_lock_hash(i, source) {
            Ok(cell_lock_hash) => cell_lock_hash,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        if cell_lock_hash[..] == lock_hash[..] {
            match get_position_by_type_args(&group_id, Source::GroupOutput) {
                Some(_) => return Ok(()),
                None => {},
            };
        }
    }

    Err(Error::GroupCellCanNotUnlock)
}

pub fn main() -> Result<(), Error> {
    let cellular_type = load_script_hash()?;

    let mut cnft_in_outputs: Vec<usize> = Vec::new(); // cnft ids
    for i in 0.. {
        match load_cell_type(i, Source::GroupOutput){
            Ok(Some(script)) => {
                if script.hash_type() != ScriptHashType::Data1.into() {
                    continue
                }
            },
            Ok(None) => continue,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        let script_hash = match load_cell_type_hash(i, Source::GroupOutput) {
            Ok(script_hash) => script_hash,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        if script_hash.unwrap_or_default() != cellular_type {
            continue;
        }

        cnft_in_outputs.push(i);
    }

    // go through inputs

    for i in 0.. {
        let script_hash = match load_cell_type_hash(i, Source::GroupInput) {
            Ok(script_hash) => script_hash,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        if script_hash.unwrap_or_default() != cellular_type {
            continue;
        }

        // process input(transfer, destruction)
        process_input(
            i,
            Source::GroupInput,
            &mut cnft_in_outputs,
            Source::GroupOutput,
        )?;
    }

    // check if any cnft cell left in outputs

    if !cnft_in_outputs.is_empty() {
        // process creation
        for index in cnft_in_outputs {
            process_creation(index, Source::GroupOutput)?;
        }
    }

    Ok(())
}
