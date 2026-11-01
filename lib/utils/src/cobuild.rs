//! Opt-in legacy CoBuild helpers for custom consumers. Core contracts do not call these.
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    high_level::{load_cell_lock, load_script_hash},
};
use spore_errors::error::Error;
use spore_types::generated::action;

pub fn check_spore_address(
    group_source: Source,
    spore_address: action::Address,
) -> Result<(), Error> {
    let address = load_cell_lock(0, group_source)?;
    let action::AddressUnion::Script(expected_script) = spore_address.to_enum();
    if address.as_slice() != expected_script.as_slice() {
        return Err(Error::SporeActionAddressesMismatch);
    }
    Ok(())
}

pub fn extract_spore_action() -> Result<action::SporeAction, Error> {
    let message = ckb_transaction_cobuild::fetch_message()
        .map_err(|_| Error::InvliadCoBuildWitnessLayout)?
        .ok_or(Error::InvliadCoBuildWitnessLayout)?;
    let script_hash = load_script_hash()?;

    let mut iter = message
        .actions()
        .into_iter()
        .filter(|value| value.script_hash().as_slice() == script_hash.as_slice());
    match (iter.next(), iter.next()) {
        (Some(action), None) => action::SporeAction::from_slice(&action.data().raw_data())
            .map_err(|_| Error::InvliadCoBuildMessage),
        _ => Err(Error::SporeActionDuplicated),
    }
}
