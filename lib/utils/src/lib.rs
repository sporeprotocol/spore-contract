#![no_std]
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::Script, prelude::*},
    debug,
    high_level::{load_cell_data, load_script, load_tx_hash, load_cell_type, QueryIter},
};

fn parse_type_opt(type_opt: &Option<Script>, predicate: &dyn Fn(&Script) -> bool) -> bool {
    match type_opt {
        Some(type_) => predicate(type_),
        None => false,
    }
}

pub fn count_cells_by_type(source: Source, predicate: &dyn Fn(&Script) -> bool) -> usize {
    QueryIter::new(load_cell_type, source)
        .filter(|type_opt| parse_type_opt(&type_opt, predicate))
        .count()
}


pub fn load_index_by_type(source: Source,type_script: &Script) -> Option<usize> {
    QueryIter::new(load_cell_type, source).position(|type_opt| {
        type_opt.map_or(false, |type_| type_.as_slice() == type_script.as_slice())
    })
}