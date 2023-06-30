#![no_std]

use molecule::prelude::Entity;
use crate::generated::cellular_types::Bool;

pub mod generated;


impl From::<generated::cellular_types::Bool> for bool {
    fn from(value: generated::cellular_types::Bool) -> bool {
         match value.as_slice().first().unwrap_or(&0) {
             0 => false,
             1 => true,
             _ => false,
         }
    }
}


impl From::<generated::cellular_types::BoolOpt> for bool {
    fn from(value: generated::cellular_types::BoolOpt) -> bool {
        if value.is_none() {
            return false
        }

        bool::from(Bool::from_slice(value.as_slice()).unwrap())
    }
}