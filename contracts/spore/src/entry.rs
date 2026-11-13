use alloc::collections::BTreeMap;
use alloc::{format, vec, vec::Vec};
use ckb_std::ckb_types::util::hash::blake2b_256;
use core::result::Result;

use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell_capacity, load_cell_lock_hash, load_script};
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
    check_spore_address, compatible_load_cluster_data, extract_spore_action,
    find_position_by_lock_hash, find_position_by_type, find_position_by_type_args_matching,
    load_self_id, verify_type_id, MIME, MUTANT_ID_LEN, MUTANT_ID_WITH_PAYMENT_LEN,
};

use crate::hash::{CLUSTER_AGENT_CODE_HASHES, CLUSTER_CODE_HASHES};

enum Operation {
    Mint,
    Transfer,
    Burn,
}

fn check_cluster_code_hash(script: &Script) -> bool {
    spore_utils::matches_code_identity(
        script,
        &CLUSTER_CODE_HASHES,
        &crate::hash::CLUSTER_TYPE_HASHES,
    )
}

fn check_agent_code_hash(script: &Script) -> bool {
    spore_utils::matches_code_identity(
        script,
        &CLUSTER_AGENT_CODE_HASHES,
        &crate::hash::CLUSTER_AGENT_TYPE_HASHES,
    )
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
    verify_extension(&mime, Operation::Mint, vec![index])?;

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
            find_position_by_type_args_matching(&cluster_id, CellDep, check_cluster_code_hash)
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
            find_position_by_type_args_matching(&cluster_id, Input, check_cluster_code_hash)
                .is_some();
        let cluster_cell_in_output =
            find_position_by_type_args_matching(&cluster_id, Output, check_cluster_code_hash)
                .is_some();

        // Condition 2: Check if cluster agent exists in Inputs & Outputs
        let agent_cell_in_input =
            find_position_by_type_args_matching(&cluster_id, Input, check_agent_code_hash)
                .is_some();
        let agent_cell_in_output =
            find_position_by_type_args_matching(&cluster_id, Output, check_agent_code_hash)
                .is_some();

        if (!cluster_cell_in_input || !cluster_cell_in_output)
            && (!agent_cell_in_input || !agent_cell_in_output)
        {
            // Condition 3: Use cluster agent in Lock Proxy mode
            if let Some(agent_index) =
                find_position_by_type_args_matching(&cluster_id, CellDep, check_agent_code_hash)
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
        verify_extension(&mime, Operation::Burn, vec![index])?;
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
        verify_extension(&mime, Operation::Transfer, vec![input_index, output_index])?;
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

fn verify_extension(mime: &MIME, op: Operation, argv: Vec<usize>) -> Result<(), Error> {
    if mime.mutants.is_empty() {
        return Ok(());
    }
    if load_script()?.hash_type() != spore_mutant::TYPE_CODE_HASH_TYPE.into() {
        return Err(Error::NativeMutantRequiresVm2);
    }
    let mut payment_map: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    for mutant_id in &mime.mutants {
        let mut selected = None;
        for (index, script) in QueryIter::new(load_cell_type, CellDep).enumerate() {
            let Some(script) = script else { continue };
            let hash: [u8; 32] = script.code_hash().unpack();
            let native = crate::hash::RUST_MUTANT_TYPE_HASHES.contains(&hash)
                && script.hash_type() == spore_mutant::TYPE_CODE_HASH_TYPE.into();
            let args = script.args().raw_data();
            if native && matches!(args.len(), 32 | 40) && args[..32] == mutant_id[..] {
                selected = Some(index);
                break;
            }
        }
        let mutant_index = selected.ok_or(Error::ExtensionCellNotInDep)?;
        if let Operation::Mint = op {
            check_payment(mutant_index, &mut payment_map)?;
        }
        use spore_mutant::{Invocation, Operation as NativeOperation};
        let (operation, input_index, output_index) = match op {
            Operation::Mint => (NativeOperation::Mint, None, Some(argv[0])),
            Operation::Transfer => (NativeOperation::Transfer, Some(argv[0]), Some(argv[1])),
            Operation::Burn => (NativeOperation::Burn, Some(argv[0]), None),
        };
        Invocation {
            operation,
            mutant_index,
            input_index,
            output_index,
        }
        .execute()
        .map_err(|error| {
            debug!("Mutant {mutant_index} failed: {error:?}");
            match error {
                spore_mutant::Error::Rejected(_) => Error::NativeMutantRejected,
                _ => Error::NativeMutantSpawnFailed,
            }
        })?;
    }
    Ok(())
}

fn capacity_sum(lock_hash: &[u8; 32], source: Source) -> Result<u64, Error> {
    let mut total = 0u64;
    for (index, hash) in QueryIter::new(load_cell_lock_hash, source).enumerate() {
        if hash == *lock_hash {
            total = total
                .checked_add(load_cell_capacity(index, source)?)
                .ok_or(Error::ExtensionPaymentOverflow)?;
        }
    }
    Ok(total)
}

fn check_payment(
    mutant_index: usize,
    payment_map: &mut BTreeMap<[u8; 32], u64>,
) -> Result<(), Error> {
    let mutant_type = load_cell_type(mutant_index, CellDep)?.ok_or(Error::ExtensionCellNotInDep)?;
    let args = mutant_type.args().raw_data();
    if args.len() > MUTANT_ID_LEN {
        if args.len() != MUTANT_ID_WITH_PAYMENT_LEN {
            return Err(Error::InvalidExtensionPaymentFormat);
        }
        let lock_hash = load_cell_lock_hash(mutant_index, CellDep)?;
        let input_capacity = capacity_sum(&lock_hash, Input)?;
        let output_capacity = capacity_sum(&lock_hash, Output)?;
        let threshold = u64::from_le_bytes(args[32..40].try_into().unwrap());
        let accumulated = payment_map.entry(lock_hash).or_default();
        *accumulated = accumulated
            .checked_add(threshold)
            .ok_or(Error::ExtensionPaymentOverflow)?;
        let required = input_capacity
            .checked_add(*accumulated)
            .ok_or(Error::ExtensionPaymentOverflow)?;
        if required > output_capacity {
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
