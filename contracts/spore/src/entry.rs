use alloc::{format, string::ToString, vec, vec::Vec};
use alloc::ffi::CString;
use core::ffi::CStr;
use core::result::Result;

use ckb_std::{ckb_constants::Source, ckb_types::prelude::*, debug, high_level::{load_cell_data, load_cell_type, QueryIter}};
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::core::ScriptHashType;
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{find_cell_by_data_hash, load_cell_data_hash, load_cell_lock, load_cell_lock_hash, load_cell_type_hash};

use spore_types::generated::spore_types::{ClusterData, SporeData};
use spore_utils::{find_position_by_type, find_position_by_lock, find_position_by_type_arg, MIME, verify_type_id, calc_capacity_sum};
use spore_constant::CodeHash::{CLUSTER_CODE_HASHES, CLUSTER_AGENT_CODE_HASHES};

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

    let content = spore_data.content();
    let content_arr = content.as_slice();


    if spore_data.content_type().is_empty() {
        return Err(Error::InvalidContentType);
    }

    // verify NFT ID
    if !verify_type_id(index, Output) {
        return Err(Error::InvalidNFTID);
    }

    let raw_content_type = spore_data.content_type();
    let content_type = raw_content_type.unpack();

    let mime = MIME::parse(content_type)?; // content_type validation
    if content_type[mime.main_type.clone()] == "multipart".as_bytes()[..] {
        // Check if boundary param exists
        let boundary_range = mime.get_param(content_type, "boundary").ok_or(Error::InvalidContentType)?;
        kmp::kmp_find(format!("--{}", alloc::str::from_utf8(&content_type[boundary_range]).or(Err(Error::Encoding))?).as_bytes(),
                      content_arr)
            .ok_or(Error::InvalidMultipartContent)?;
    }

    if spore_data.cluster_id().to_opt().is_some() {
        // check if cluster cell in deps
        let cluster_id = spore_data.cluster_id().to_opt().unwrap_or_default();
        let cluster_id = cluster_id.as_slice();
        let filter_fn: fn(&[u8; 32]) -> bool = |x| -> bool { CLUSTER_CODE_HASHES.contains(x) };
        let filter_fn2: fn(&[u8; 32]) -> bool = |x| -> bool { CLUSTER_AGENT_CODE_HASHES.contains(x) };
        let cell_dep_index = find_position_by_type_arg(&cluster_id, CellDep, Some(filter_fn)).ok_or(Error::ClusterCellNotInDep)?;

        let raw_cluster_data = load_cell_data(cell_dep_index, CellDep)?;
        let cluster_data = ClusterData::from_compatible_slice(raw_cluster_data.as_slice()).unwrap_or_default(); // the cluster contract guarantees the cluster data will always be correct once created
        if cluster_data.mutant_id().is_some() {
            let mutant_id  = cluster_data.mutant_id().to_opt().unwrap_or_default();
            let mutant_id = mutant_id.unpack();
            let mut mutant_verify_passed = false;
            for mutant in mime.mutants.iter() {
                if mutant[0..32] == mutant_id[0..32] {
                    mutant_verify_passed = true;
                    break
                }
            }
            if !mutant_verify_passed { // required mutant does not applied
                return Err(Error::ClusterRequiresMutantApplied)
            }
        }

        // Condition 1: Check if cluster exist in Inputs & Outputs
        return if find_position_by_type_arg(&cluster_id, Input, Some(filter_fn)).is_some()
            && find_position_by_type_arg(&cluster_id, Output, Some(filter_fn)).is_some() {
            Ok(())
        } // Condition 2: Check if cluster agent in Inputs & Outputs
        else if find_position_by_type_arg(&cluster_id, Input, Some(filter_fn2)).is_some()
            && find_position_by_type_arg(&cluster_id, Output, Some(filter_fn2)).is_some() {
            Ok(())
        } // Condition 3: Use cluster agent by lock proxy
        else if let Some(agent_index) = find_position_by_type_arg(&cluster_id, CellDep, Some(filter_fn2)) {
            let agent_lock_hash =  load_cell_lock_hash(agent_index, CellDep)?;
            find_position_by_lock(&agent_lock_hash, Output).ok_or(Error::ClusterOwnershipVerifyFailed)?;
            find_position_by_lock(&agent_lock_hash, Input).ok_or(Error::ClusterOwnershipVerifyFailed)?;
            Ok(())
        }
        else {
            // Condition 4: Check if Lock Proxy exist in Inputs & Outputs
            let cluster_lock_hash = load_cell_lock_hash(cell_dep_index, CellDep)?;
            find_position_by_lock(&cluster_lock_hash, Output).ok_or(Error::ClusterOwnershipVerifyFailed)?;
            find_position_by_lock(&cluster_lock_hash, Input).ok_or(Error::ClusterOwnershipVerifyFailed)?;
            Ok(())
        }
    }

    if !mime.mutants.is_empty() {
        verify_extension(&mime, 0, vec![index as u8])?;
    }

    Ok(())
}


fn process_destruction() -> Result<(), Error> {
    //destruction
    let spore_data = load_spore_data(0, GroupInput)?;

    let content_type_bytes = spore_data.content_type();
    let content_type = content_type_bytes.unpack();
    let mime = MIME::parse(content_type)?;
    if mime.immortal {
        // true destroy a immortal nft
        return Err(Error::DestroyImmortalNFT);
    }

    if !mime.mutants.is_empty() {
        let type_hash = load_cell_type(0, GroupInput)?.unwrap_or_default();
        let index = find_position_by_type(type_hash.as_slice(), Input).ok_or(Error::IndexOutOfBound)?;
        verify_extension(&mime, 2, vec![index as u8])?;
    }

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    // found same NFT in output, this is a transfer
    // check no field was modified
    let input_data = load_spore_data(0, GroupInput)?;
    let output_data = load_spore_data(0, GroupOutput)?;

    if input_data.as_slice()[..] != output_data.as_slice()[..] {
        return Err(Error::ModifyPermanentField);
    }


    let content_type_bytes = input_data.content_type();
    let content_type = content_type_bytes.unpack();
    let mime = MIME::parse(content_type)?;

    if !mime.mutants.is_empty() {
        let type_hash = load_cell_type(0, GroupInput)?.unwrap_or_default();
        let input_index = find_position_by_type(type_hash.as_slice(), Input).ok_or(Error::IndexOutOfBound)?;
        let output_index = find_position_by_type(type_hash.as_slice(), Output).ok_or(Error::IndexOutOfBound)?;
        verify_extension(&mime, 1, vec![input_index as u8, output_index as u8])?;
    }

    Ok(())
}

fn verify_extension(mime: &MIME, op: usize, argv: Vec<u8>) -> Result<(), Error> {
    for mutant in mime.mutants.iter() {
        let ext_pos =  QueryIter::new(load_cell_type, CellDep)
            .position(|script| {
                match script {
                    Some(script) => {
                        mutant[..32] == script.args().as_slice()[..32]
                    }
                    None => false,
                }
            });
        match ext_pos {
            None => return Err(Error::ExtensionCellNotInDep),
            Some(ext_pos) => {

                if op == 0 {
                    check_payment(ext_pos)?;
                }

                let ext_pos = ext_pos as u8;
                let code_hash = load_cell_data_hash(ext_pos.into(), CellDep)?;
                match op {
                    0 | 2 => {
                        ckb_std::high_level::exec_cell(&code_hash, ScriptHashType::Data1,
                                                       &[
                                                           CStr::from_bytes_with_nul([b'0', 0].as_slice()).unwrap_or_default(),
                                                           CStr::from_bytes_with_nul([b'0' + ext_pos, 0].as_slice()).unwrap_or_default(),
                                                           CStr::from_bytes_with_nul([b'0' + argv[0], 0].as_slice()).unwrap_or_default(),
                                                       ])?;
                    },
                    1 => {
                        ckb_std::high_level::exec_cell(&code_hash, ScriptHashType::Data1,
                                                       &[
                                                           CStr::from_bytes_with_nul([b'0', 0].as_slice()).unwrap_or_default(),
                                                           CStr::from_bytes_with_nul([b'0' + ext_pos, 0].as_slice()).unwrap_or_default(),
                                                           CStr::from_bytes_with_nul([b'0' + argv[0], 0].as_slice()).unwrap_or_default(),
                                                           CStr::from_bytes_with_nul([b'0' + argv[1], 0].as_slice()).unwrap_or_default(),
                                                       ])?;

                    },
                    _ => unreachable!()
                }
            }
        }
    }
    Ok(())
}

fn check_payment(ext_pos: usize) -> Result<(), Error> {
    let ext_script = load_cell_type(ext_pos, CellDep)?.unwrap_or_default();
    let ext_arg = ext_script.args();
    if ext_arg.len() == 33 { // we need a payment
        let lock = load_cell_lock_hash(ext_pos, CellDep)?;

        let input_capacity = calc_capacity_sum(&lock,Input);
        let output_capacity = calc_capacity_sum(&lock,Output);
        let minimal_payment = 10u128.pow(ext_arg.get(32).unwrap_or_default().as_slice()[0] as u32);
        if input_capacity + minimal_payment < output_capacity {
            return Err(Error::ExtensionPaymentNotEnough)
        }
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
