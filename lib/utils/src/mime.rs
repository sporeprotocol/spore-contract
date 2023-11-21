use alloc::boxed::Box;
use alloc::str;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::ops::Add;
use ckb_std::ckb_types::prelude::Entity;
use ckb_std::debug;
use ckb_std::error::SysError;
use spore_types::generated::spore_types::Bytes;
use crate::mime::MIMEErrorCode::Illformed;
use crate::mime::ParamType::Generic;

#[repr(u64)]
pub enum MIMEErrorCode {
    Illformed = 100,
    InvaliMainType = 101,
    InvalidSubType = 102,
    InvalidParams = 103,
    InvalidParamValue = 104,
    MutantIDNotValid = 105,
}

impl Into<u64> for MIMEErrorCode {
    fn into(self) -> u64 {
        self as u64
    }
}

type RangePair = core::ops::Range<usize>;


#[derive(Debug, Clone)]
enum ParamType {
    Generic(RangePair),
    Immortal(RangePair),
    Mutant(RangePair),
}

#[derive(Debug, Clone)]
pub struct MIME {
    pub main_type: RangePair,
    pub sub_type: RangePair,
    params: Vec<(RangePair, RangePair)>,
    pub mutants: Vec<[u8;32]>,
    pub immortal: bool,
}



impl MIME {
    pub fn parse(raw_content_type: &[u8]) -> Result<MIME, SysError> {
        let content_type = match str::from_utf8(raw_content_type) {
            Ok(x) => x,
            _ => return Err(SysError::Unknown(Illformed.into())),
        }.trim_start().trim_end();
        Self::str_parse(&content_type)
    }

    pub fn str_parse(content_type: &str) -> Result<Self, SysError> {
        debug!("Content type is: {}", content_type);
        // main_type.len() + '/' + sub_type.len() + '+' +
        let (main_type, right) = match content_type.find('/') {
            Some(pos) => (0usize..pos, pos..content_type.len()),
            _ => return Err(SysError::Unknown(MIMEErrorCode::Illformed.into())),
        };

        if !is_restricted_name(&content_type[main_type.clone()]) {
            return Err(SysError::Unknown(MIMEErrorCode::InvaliMainType.into()));
        }

        if !content_type[right.clone()].chars().any(is_restricted_char) {
            return Err(SysError::Unknown(MIMEErrorCode::Illformed.into()));
        }

        let sub_end = content_type[right.clone()].find(';').unwrap_or(right.len()) + main_type.end;
        let sub_type = main_type.end+1..sub_end;

        let mut vec = Vec::new();
        let right_part = &content_type[sub_end..];
        let mut offset = sub_end;
        let mut mutants = Vec::new();
        let mut immortal = false;
        while let Some((name_range, value_range, new_offset)) = parse_param(content_type, offset)? {
            match name_range {
                ParamType::Mutant(name_range) => {
                    vec.push((name_range, value_range.clone()));
                    let value = &content_type[value_range];
                    for mutant_id in value.split(',') {
                        let mutant_id_str = mutant_id.trim_matches(is_ows);
                        if mutant_id_str.len() != 32 {
                            return Err(SysError::Unknown(MIMEErrorCode::MutantIDNotValid.into()));
                        }
                        let mut mutant_id = [0u8;32];
                        mutant_id.copy_from_slice(mutant_id_str.as_bytes());
                        mutants.push(mutant_id);
                    }
                },
                ParamType::Generic(name_range) => {
                    vec.push((name_range, value_range));
                },
                ParamType::Immortal(name_range) => {
                    immortal = &content_type[value_range.clone()] == "true";
                    vec.push((name_range, value_range));
                }
            }
            offset = new_offset;
        }

        let mime_type = MIME {
            main_type: main_type,
            sub_type: sub_type,
            params: vec,
            mutants,
            immortal
        };

        Ok(mime_type)
    }

    pub fn params(&self) -> &Vec<(RangePair, RangePair)> {
        &self.params
    }

    pub fn mut_params(&mut self) -> &mut Vec<(RangePair, RangePair)> {
        &mut self.params
    }

    pub fn get_param(&self, content_type: &[u8], param: &str) -> Option<RangePair> {
        for (param_range, value_range) in self.params.iter() {
            if content_type[param_range.clone()] == param.as_bytes()[..] {
                return Some(value_range.clone())
            }
        }
        None
    }

    pub fn verify_param(&self, content_type: &[u8], param: &str, value: &[u8]) -> bool {
        for (param_range, value_range) in self.params.iter() {
            if content_type[param_range.clone()] == param.as_bytes()[..] {
                return content_type[value_range.clone()] == value[..]
            }
        }
        false
    }
}

pub fn is_restricted_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '*')
        && is_restricted_str(s)
}

pub fn is_restricted_name_patched(s: &str) -> bool {
    s == "mutant[]" || is_restricted_name(s)
}

pub fn is_restricted_value_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+' | '%' | '*' | '\'' | ','
        )
}

pub fn is_restricted_str(s: &str) -> bool {
    s.chars().all(is_restricted_char)
}

pub fn is_restricted_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+' | '%' | '*' | '\''
        )
}

pub const fn is_ows(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn parse_param(source: &str, offset: usize) -> Result<Option<(ParamType, RangePair, usize)>, SysError> {
    if offset >= source.len() {
        return Ok(None);
    }
    let s= &source[offset..];
    let (lhs, rhs) = match s.split_once(';') {
        Some((lhs, rhs)) if lhs.chars().all(is_ows) && rhs.chars().all(is_ows) => {
            return Ok(None)
        },
        Some((lhs, rhs)) if lhs.chars().all(is_ows) => (lhs, rhs),
        _ if s.chars().all(is_ows) => return Ok(None),
        _ => return Err(SysError::Unknown(MIMEErrorCode::InvalidParams.into())),
    };

    let (name, value) = match rhs.split_once('=') {
        Some((name, value_maybe)) => {
            match value_maybe.split_once(';') {
                None => (name, value_maybe),
                Some((value_maybe_lhs, _)) => (name, value_maybe_lhs),
            }
        },
        _ => return Err(SysError::Unknown(MIMEErrorCode::InvalidParams.into())),
    };

    let key_trimmed = name.trim_start_matches(is_ows).len();
    let key_start = lhs.len() + 1 + name.len() - key_trimmed;
    let key_range = key_start + offset ..key_start + offset + key_trimmed;
    if !is_restricted_name_patched(&source[key_range.clone()]) {
        return Err(SysError::Unknown(MIMEErrorCode::InvalidParams.into()));
    }
    let key = match &source[key_range.clone()] {
        "immortal" => {
            ParamType::Immortal(key_range.clone())
        },
        "mutant[]" => {
            ParamType::Mutant(key_range.clone())
        },
        _ => {
            ParamType::Generic(key_range.clone())
        }
    };
    let value_start = key_range.end + 1;
    if let Some(value) = value.strip_prefix('\"') {
        let value_end = value_start + parse_quoted_value(value)? + 1;
        let value_range = value_start..value_end;
        Ok(Some((key.clone(), value_range.clone(), value_end)))
    } else {
        let value_end = value_start + value.chars().take_while(|&c| is_restricted_value_char(c)).map(char::len_utf8).sum::<usize>();
        let value_range = value_start..value_end;
        Ok(Some((key.clone(), value_range.clone(), value_end)))
    }
}

pub fn parse_quoted_value(s: &str) -> Result<usize, SysError> {
    let mut len = 0;
    let mut escaped = false;
    for c in s.chars() {
        len += c.len_utf8();
        match c {
            _ if escaped => {
                escaped = false;
            }
            '\\' => {
                escaped = true;
            }
            '"' => return Ok(len),
            '\n' => return Err(SysError::Unknown(MIMEErrorCode::InvalidParamValue.into())),
            _ => (),
        }
    }
    Err(SysError::Unknown(MIMEErrorCode::InvalidParamValue.into()))
}

#[test]
fn test_basic() {
    assert!(MIME::str_parse("image/png").is_ok());
    assert!(MIME::str_parse("image/png;immortal=true").is_ok());
    assert!(MIME::str_parse("image/png;immortal=true;mutant[]=2,3,4,5").is_ok());
    assert!(MIME::str_parse("image/png;immortal=true;mutant[]=2,3,4,5").expect("mutant verify_param")
        .verify_param(b"image/png;immortal=true;mutant[]=2,3,4,5", "mutant[]", b"2,3,4,5"));
    assert!(MIME::str_parse("image/png;immortal=true;mutant[]=2,3,4,5").expect("mutant verify_param")
        .verify_param(b"image/png;immortal=true;mutant[]=2,3,4,5", "immortal", b"true"));
    assert!(MIME::str_parse("image/").is_err());
    assert!(MIME::str_parse("image/;").is_err());
    assert!(MIME::str_parse("/;").is_err());
    assert!(MIME::str_parse(";").is_err());
    assert!(MIME::str_parse("").is_err());
}
