use alloc::{string::ToString, vec::Vec};
use alloc::string::String;
use core::char::decode_utf16;
use core::result::Result;

use ckb_std::high_level::{decode_hex, load_cell_type_hash};
use ckb_std::{ckb_constants::Source, ckb_types::prelude::*, debug, high_level::{load_cell_data, load_cell_type, load_script_hash, QueryIter}};

use spore_types::generated::spore_types::{Bytes, BytesBuilder, BytesOpt, BytesOptBuilder, SporeData};
use spore_utils::{verify_type_id, MIME};

use crate::error::Error;

fn load_nft_data(index: usize, source: Source) -> Result<SporeData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let nft_data = SporeData::from_slice(raw_data.as_slice()).map_err(|_| Error::InvalidNFTData)?;
    Ok(nft_data)
}

fn get_position_by_type_args(args: &[u8], source: Source) -> Option<usize> {
    QueryIter::new(load_cell_type, source)
        .position(|x| {
            let lhs_args= x.unwrap_or_default().args();
            lhs_args.as_slice()[..] == args[..]
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

    let _ = MIME::parse(nft_data.content_type()).map_err(|_| Error::InvalidContentType)?; // content_type validation

    if nft_data.cluster().to_opt().is_some() {
        // need to check if group cell in deps
        let group_id = nft_data.cluster().to_opt().unwrap_or_default();
        let group_id = group_id.as_slice();


        get_position_by_type_args(&group_id, Source::CellDep).ok_or(Error::ClusterCellNotInDep)?;
        get_position_by_type_args(&group_id, Source::Input).ok_or(Error::ClusterCellCanNotUnlock)?;
        get_position_by_type_args(&group_id, Source::Output).ok_or(Error::ClusterCellCanNotUnlock)?;
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let spore_type = load_script_hash()?;
    let mut cnft_in_outputs: Vec<usize> = QueryIter::new(load_cell_type_hash, Source::GroupOutput)
        .enumerate()
        .filter(|(_, type_hash)| spore_type[..] == type_hash.unwrap_or_default()[..] )
        .map(|(pos, _)| pos
        ).collect();

    // go through inputs, looking for cell matched with code hash

    QueryIter::new(load_cell_type_hash, Source::Input)
        .enumerate()
        .filter(|(_, type_hash)| spore_type[..] == type_hash.unwrap_or_default()[..])
        .try_for_each(|(pos, _)|
            // process every matched spore cell in input
            process_input(pos,
                          Source::Input,
                          &mut cnft_in_outputs,
                          Source::Output))?;

    if !cnft_in_outputs.is_empty() {
        for index in cnft_in_outputs {
            // process matched spore creation cells in output
            process_creation(index, Source::Output)?
        }
    }

    Ok(())
}
