#![no_std]

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use ckb_std::ckb_types::prelude::{Builder, Entity, Pack, PackVec};
use ckb_std::error::SysError;
use spore_types::generated::spore_types::{Bytes, SporeData};

#[derive(Debug)]
pub struct MIME {
    main_type: String,
    sub_type: String,
    params: BTreeMap<String, String>,
}

impl MIME {
    pub fn parse(raw_content_type: Bytes) -> Result<Self, SysError> {
        let content_type = match String::from_utf8(raw_content_type.as_slice().to_vec()) {
            Ok(x) => x,
            _ => return Err(SysError::Encoding),
        };

        Self::str_parse(content_type)
    }

    pub fn str_parse(content_type: String) -> Result<Self, SysError> {
        let slash_pos = content_type.find('/').ok_or(SysError::Encoding)?;

        if slash_pos + 1 == content_type.len() {
            return Err(SysError::Encoding);
        }

        let param_start_pos = content_type.find(';');

        let has_param_part = match param_start_pos {
            None => false,
            Some(pos) => pos != content_type.len(),
        };

        if slash_pos == content_type.len() || // empty subtype
            slash_pos == 0
        {
            return Err(SysError::Encoding);
        }

        let type_part = if has_param_part {
            content_type
                .split_at(param_start_pos.unwrap())
                .0
                .to_string()
        } else {
            content_type.clone()
        };

        let param_part = if has_param_part {
            Some(content_type.split_at(param_start_pos.unwrap()).1)
        } else {
            None
        };

        let (main_type, sub_type) = type_part.split_at(slash_pos);

        let mut mime_type = MIME {
            main_type: main_type.to_string(),
            sub_type: sub_type.to_string(),
            params: BTreeMap::new(),
        };

        if has_param_part {
            let param_part = param_part.unwrap().trim_start_matches(';');
            for param in param_part.split(';') {
                let mut param_parts = param.splitn(2, '=');
                let key = param_parts.next();
                if key.is_none() {
                    break;
                }
                let key = key.unwrap().trim().to_string();
                let value = param_parts.next();
                if value.is_none() {
                    return Err(SysError::Encoding);
                }
                let value = value.unwrap().trim().trim_matches('\"').to_string();
                mime_type.mut_params().insert(key, value);
            }
        }
        Ok(mime_type)
    }

    pub fn params(&self) -> &BTreeMap<String, String> {
        &self.params
    }

    pub fn mut_params(&mut self) -> &mut BTreeMap<String, String> {
        &mut self.params
    }
}

#[test]
fn test_basic() {
    assert!(MIME::str_parse(String::from("image/png")).is_ok());
    assert!(MIME::str_parse(String::from("image/png;immortal=true")).is_ok());
    assert!(MIME::str_parse(String::from("image/")).is_err());
    assert!(MIME::str_parse(String::from("image/;")).is_err());
    assert!(MIME::str_parse(String::from("/;")).is_err());
    assert!(MIME::str_parse(String::from(";")).is_err());
    assert!(MIME::str_parse(String::from("")).is_err());
}
