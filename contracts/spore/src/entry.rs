use alloc::collections::BTreeMap;
use alloc::{format, vec, vec::Vec};
use ckb_std::ckb_types::util::hash::blake2b_256;
use core::ffi::CStr;
use core::result::Result;

use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::core::ScriptHashType;
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell_lock_hash, load_script};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    debug,
    high_level::{load_cell_data, load_cell_type, QueryIter},
};

use spore_errors::error::Error;
use spore_types::generated::action;
use spore_types::generated::spore::SporeData;
use spore_utils::{
    calc_capacity_sum, check_spore_address, compatible_load_cluster_data, extract_spore_action,
    find_position_by_lock_hash, find_position_by_type, find_position_by_type_args, load_self_id,
    verify_type_id, MIME, MUTANT_ID_LEN, MUTANT_ID_WITH_PAYMENT_LEN,
};

use crate::hash::{CLUSTER_AGENT_CODE_HASHES, CLUSTER_CODE_HASHES};

enum Operation {
    Mint,
    Transfer,
    Burn,
}

fn check_cluster_code_hash(code_hash: &[u8; 32]) -> bool {
    CLUSTER_CODE_HASHES.contains(code_hash)
}

fn check_agent_code_hash(code_hash: &[u8; 32]) -> bool {
    CLUSTER_AGENT_CODE_HASHES.contains(code_hash)
}

fn load_spore_data(index: usize, source: Source) -> Result<SporeData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let spore_data = SporeData::from_compatible_slice(raw_data.as_slice())
        .map_err(|_| Error::InvalidSporeData)?;
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

    // verify Spore ID
    let Some(spore_id) = verify_type_id(index) else {
        return Err(Error::InvalidSporeID);
    };

    // content_type validation
    let content_type = spore_data.content_type().raw_data();
    let mime = MIME::parse(&content_type)?;
    verify_extension(&mime, Operation::Mint, vec![index as u8])?;

    // Spore supports [MIME-multipart](https://datatracker.ietf.org/doc/html/rfc1521#section-7.2).
    //
    // The Multipart Content-Type is used to represent a document that is comprised of multiple
    // parts, each of which may have its own individual MIME type
    if content_type[mime.main_type.clone()] == "multipart".as_bytes()[..] {
        // Check if boundary param exists
        // The Content-Type field for multipart entities requires one parameter, "boundary", which
        // is used to specify the encapsulation boundary. See Appendix C of rfc1521 for a complex
        // multipart example.
        debug!("check mime multipart specification");
        let boundary_range = mime
            .get_param(&content_type, "boundary")?
            .ok_or(Error::InvalidContentType)?;
        kmp::kmp_find(
            format!(
                "--{}",
                alloc::str::from_utf8(&content_type[boundary_range])
                    .or(Err(Error::BoundaryEncoding))?
            )
            .as_bytes(),
            content_arr,
        )
        .ok_or(Error::InvalidMultipartContent)?;
    }

    // check in Cluster mode
    if let Some(cluster_id) = spore_data.cluster_id().to_opt() {
        debug!("check in cluster mode");
        // check if cluster cell is in deps
        let cluster_id = cluster_id.raw_data();
        let cell_dep_index =
            find_position_by_type_args(&cluster_id, CellDep, Some(check_cluster_code_hash))
                .ok_or(Error::ClusterCellNotInDep)?;

        // the cluster contract guarantees the cluster data will always be correct once created
        let raw_cluster_data = load_cell_data(cell_dep_index, CellDep)?;
        let cluster_data = compatible_load_cluster_data(&raw_cluster_data)?;

        // check in Mutant mode
        if let Some(mutant_id) = cluster_data.mutant_id().to_opt() {
            let mutant_verify_passed = mime
                .mutants
                .iter()
                .any(|mutant| mutant == mutant_id.raw_data().as_ref());
            if !mutant_verify_passed {
                // required mutant does not applied
                return Err(Error::ClusterRequiresMutantApplied);
            }
        }

        // Condition 1: Check if cluster exists in Inputs & Outputs
        let cluster_cell_in_input =
            find_position_by_type_args(&cluster_id, Input, Some(check_cluster_code_hash)).is_some();
        let cluster_cell_in_output =
            find_position_by_type_args(&cluster_id, Output, Some(check_cluster_code_hash))
                .is_some();

        // Condition 2: Check if cluster agent exists in Inputs & Outputs
        let agent_cell_in_input =
            find_position_by_type_args(&cluster_id, Input, Some(check_agent_code_hash)).is_some();
        let agent_cell_in_output =
            find_position_by_type_args(&cluster_id, Output, Some(check_agent_code_hash)).is_some();

        if (!cluster_cell_in_input || !cluster_cell_in_output)
            && (!agent_cell_in_input || !agent_cell_in_output)
        {
            // Condition 3: Use cluster agent in Lock Proxy mode
            if let Some(agent_index) =
                find_position_by_type_args(&cluster_id, CellDep, Some(check_agent_code_hash))
            {
                debug!("check in agent mode");
                let agent_lock_hash = load_cell_lock_hash(agent_index, CellDep)?;
                find_position_by_lock_hash(&agent_lock_hash, Output)
                    .ok_or(Error::ClusterOwnershipVerifyFailed)?;
                find_position_by_lock_hash(&agent_lock_hash, Input)
                    .ok_or(Error::ClusterOwnershipVerifyFailed)?;
            } else {
                debug!("check in lock proxy mode");
                // Condition 4: Check if Lock Proxy exist in Inputs & Outputs
                let cluster_lock_hash = load_cell_lock_hash(cell_dep_index, CellDep)?;
                find_position_by_lock_hash(&cluster_lock_hash, Output)
                    .ok_or(Error::ClusterOwnershipVerifyFailed)?;
                find_position_by_lock_hash(&cluster_lock_hash, Input)
                    .ok_or(Error::ClusterOwnershipVerifyFailed)?;
            }
        }
    }

    // check co-build action @lyk
    let action::SporeActionUnion::MintSpore(mint) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if mint.spore_id().as_slice() != spore_id
        || mint.data_hash().as_slice() != blake2b_256(spore_data.as_slice())
    {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupOutput, mint.to())?;

    Ok(())
}

fn process_destruction() -> Result<(), Error> {
    let spore_data = load_spore_data(0, GroupInput)?;
    let content_type = spore_data.content_type().raw_data();

    let mime = MIME::parse(&content_type)?;
    if mime.immortal {
        // true destroy a immortal nft
        return Err(Error::DestroyImmortalNFT);
    }

    if !mime.mutants.is_empty() {
        let spore_type = load_script()?;
        let index = find_position_by_type(&spore_type, Input).ok_or(Error::IndexOutOfBound)?;
        verify_extension(&mime, Operation::Burn, vec![index as u8])?;
    }

    // check co-build action @lyk
    let action::SporeActionUnion::BurnSpore(burn) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if burn.spore_id().as_slice() != &load_self_id()? {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, burn.from())?;

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    // found same NFT in output, this is a transfer, check no field was modified
    let input_data = load_spore_data(0, GroupInput)?;
    let output_data = load_spore_data(0, GroupOutput)?;

    if input_data.as_slice()[..] != output_data.as_slice()[..] {
        return Err(Error::ModifySporePermanentField);
    }

    let content_type = input_data.content_type().raw_data();
    let mime = MIME::parse(&content_type)?;

    if !mime.mutants.is_empty() {
        let spore_type = load_script()?;
        let input_index =
            find_position_by_type(&spore_type, Input).ok_or(Error::IndexOutOfBound)?;
        let output_index =
            find_position_by_type(&spore_type, Output).ok_or(Error::IndexOutOfBound)?;
        verify_extension(
            &mime,
            Operation::Transfer,
            vec![input_index as u8, output_index as u8],
        )?;
    }

    // check co-build action @lyk
    let action::SporeActionUnion::TransferSpore(transfer) = extract_spore_action()?.to_enum()
    else {
        return Err(Error::SporeActionMismatch);
    };
    if transfer.spore_id().as_slice() != &load_self_id()? {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, transfer.from())?;
    check_spore_address(GroupOutput, transfer.to())?;

    Ok(())
}

fn verify_extension(mime: &MIME, op: Operation, argv: Vec<u8>) -> Result<(), Error> {
    let mut payment_map: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    let mut extension_hash = [0u8; 32];
    for mutant_id in mime.mutants.iter() {
        let mutant_index =
            QueryIter::new(load_cell_type, CellDep).position(|script| match script {
                Some(script) => {
                    extension_hash = script.code_hash().unpack();
                    if crate::hash::MUTANT_CODE_HASHES.contains(&extension_hash) {
                        return mutant_id[..] == script.args().raw_data()[..32];
                    }
                    false
                }
                None => false,
            });
        match mutant_index {
            None => return Err(Error::ExtensionCellNotInDep),
            Some(mutant_index) => {
                // mint spore should pay if payment set
                if let Operation::Mint = op {
                    check_payment(mutant_index, &mut payment_map)?;
                }

                debug!("run mutant_id({mutant_index}): {mutant_id:?} <= {extension_hash:?}");
                match op {
                    Operation::Mint | Operation::Burn => {
                        ckb_std::high_level::exec_cell(
                            &extension_hash,
                            ScriptHashType::Data1,
                            &[
                                CStr::from_bytes_with_nul([b'0', 0].as_slice()).unwrap_or_default(),
                                CStr::from_bytes_with_nul(
                                    [b'0' + mutant_index as u8, 0].as_slice(),
                                )
                                .unwrap_or_default(),
                                CStr::from_bytes_with_nul([b'0' + argv[0], 0].as_slice())
                                    .unwrap_or_default(),
                            ],
                        )?;
                    }
                    Operation::Transfer => {
                        ckb_std::high_level::exec_cell(
                            &extension_hash,
                            ScriptHashType::Data1,
                            &[
                                CStr::from_bytes_with_nul([b'0', 0].as_slice()).unwrap_or_default(),
                                CStr::from_bytes_with_nul(
                                    [b'0' + mutant_index as u8, 0].as_slice(),
                                )
                                .unwrap_or_default(),
                                CStr::from_bytes_with_nul([b'0' + argv[0], 0].as_slice())
                                    .unwrap_or_default(),
                                CStr::from_bytes_with_nul([b'0' + argv[1], 0].as_slice())
                                    .unwrap_or_default(),
                            ],
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_payment(
    mutant_index: usize,
    payment_map: &mut BTreeMap<[u8; 32], u64>,
) -> Result<(), Error> {
    let mutant_type = load_cell_type(mutant_index, CellDep)?.unwrap_or_default();
    let args = mutant_type.args().raw_data();
    // CAUTION: only check bytes in [32, 40) pattern, leave room for user customization
    if args.len() > MUTANT_ID_LEN {
        if args.len() < MUTANT_ID_WITH_PAYMENT_LEN {
            return Err(Error::InvalidExtensionPaymentFormat);
        }
        // we need a payment
        let self_lock_hash = load_cell_lock_hash(0, GroupOutput)?;
        let mutant_lock_hash = load_cell_lock_hash(mutant_index, CellDep)?;

        let input_capacity = calc_capacity_sum(&self_lock_hash, Input);
        let output_capacity = calc_capacity_sum(&mutant_lock_hash, Output);
        let minimal_payment = {
            let range = MUTANT_ID_LEN..MUTANT_ID_WITH_PAYMENT_LEN;
            let threshold = u64::from_le_bytes(args[range].try_into().unwrap_or_default());
            let payment_threshold = payment_map.entry(mutant_lock_hash).or_default();
            *payment_threshold += threshold;
            *payment_threshold
        };
        if input_capacity + minimal_payment > output_capacity {
            return Err(Error::ExtensionPaymentNotEnough);
        }
    }
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let spore_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if spore_in_output.len() > 1 {
        return Err(Error::ConflictCreation);
    }

    let spore_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if spore_in_input.len() > 1 {
        return Err(Error::MultipleSpend);
    }

    match (spore_in_input.len(), spore_in_output.len()) {
        (0, 1) => {
            // find it's index in Output
            let output_index =
                find_position_by_type(&spore_in_output[0], Output).ok_or(Error::IndexOutOfBound)?;
            return process_creation(output_index);
        }
        (1, 0) => {
            return process_destruction();
        }
        (1, 1) => {
            return process_transfer();
        }
        _ => unreachable!(),
    }
}
