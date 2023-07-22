use alloc::{ string::ToString, vec::Vec };
use alloc::string::String;
use core::result::Result;
use ckb_std::{ckb_constants::Source, ckb_types::{prelude::*, util::hash::Blake2bBuilder}, high_level::{load_cell_data, load_cell_type, load_cell_lock_hash, load_cell_type_hash, load_script_hash, QueryIter}, error::SysError, debug};
use ckb_std::ckb_types::core::ScriptHashType;
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell, load_cell_lock};

use spore_types::generated::spore_types::{Bytes, SporeData};

use crate::error::Error;
use spore_utils::{MIME, type_hash_filter_builder, verify_type_id};

fn load_nft_data(index: usize, source: Source) -> Result<SporeData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let nft_data = SporeData::from_slice(raw_data.as_slice()).map_err(|_| Error::InvalidNFTData)?;
    Ok(nft_data)
}

fn get_position_by_type_args(args: &Bytes, source: Source) -> Option<usize> {
    QueryIter::new(load_cell_type, source).position(|x|
        x.unwrap_or_default().args().as_slice()[..] == args.as_slice()[..]
    )
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

            if nft_data.as_slice()[..] != output_nft_data.as_slice()[..] {

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


    let mime = MIME::parse(nft_data.content_type()).map_err(|_|Error::InvalidContentType)?; // content_type validation
    debug!("Mime Params: {:?}", mime.params());

    if nft_data.cluster().is_some() {
        // need to check if group cell in deps
        let group_id = nft_data.cluster().to_opt().unwrap();
        let group_cell_pos = get_position_by_type_args(&group_id, Source::CellDep);

        if group_cell_pos.is_none() {
            return Err(Error::ClusterCellNotInDep);
        }

        let group_cell_pos = group_cell_pos.unwrap();

        // check ownership
        let lock_args = load_cell_lock(group_cell_pos, Source::CellDep)?.args();

        verify_group_cell(&lock_args.as_slice(), group_id, Source::GroupInput, Source::GroupOutput)?;
    }

    Ok(())
}

fn verify_group_cell(lock_args: &[u8], group_id: Bytes, input_source: Source, output_source: Source) -> Result<(), Error> {
    match QueryIter::new(load_cell, input_source)
        .filter(|cell| {
            cell.lock().args().as_slice()[..] == lock_args[..]
        })
        .filter(|cell| {
            match cell.type_().to_opt() {
                Some(script) => {
                    script.args().as_slice()[..] == group_id.as_slice()[..]
                },
                _ => false
            }
        }).find(|cell| {
        QueryIter::new(load_cell, output_source)
            .position(|output_cell| {
                if let Some(type_script) = output_cell.type_().to_opt() {
                    type_script.args().as_slice()[..] == group_id.as_slice()[..]
                        && cell.lock().args().as_slice()[..] == output_cell.lock().args().as_slice()[..]
                } else {
                    false
                }
            }).is_some()

    })
    {
        Some(_) =>  Ok(()),
        _ =>  Err(Error::ClusterCellCanNotUnlock)
    }
}

pub fn main() -> Result<(), Error> {
    let spore_type = load_script_hash()?;

    let filter_for_spore_type = type_hash_filter_builder(spore_type, ScriptHashType::Data1);

    let mut cnft_in_outputs = QueryIter::new(load_cell_type, Source::GroupOutput)
        .enumerate().filter(|(_, script)| {
        filter_for_spore_type(script)
    }).map(|(pos, _)| pos).collect();

    // go through inputs

    QueryIter::new(load_cell_type, Source::GroupInput)
        .enumerate().filter(|(_, script)|filter_for_spore_type(script))
        .try_for_each(|(pos, _)|
            process_input(pos,
                          Source::GroupInput,
                          &mut cnft_in_outputs,
                          Source::GroupOutput)
        )?;

    // check if any cnft cell left in outputs

    if !cnft_in_outputs.is_empty() {
        // process creation
        for index in cnft_in_outputs {
            debug!("Process Creation for index: {}", index);
            process_creation(index, Source::GroupOutput)?;
        }
    }

    Ok(())
}
