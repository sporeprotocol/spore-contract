// note: impelemtation refers to https://github.com/hyperium/mime

use core::ops::Range;

use alloc::ffi::CString;
use alloc::str;
use alloc::vec::Vec;
use ckb_std::debug;
use ckb_std::high_level::decode_hex;
use spore_errors::error::Error;

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
    pub mutants: Vec<[u8; 32]>,
    pub immortal: bool,
    params: Vec<(RangePair, RangePair)>,
}

impl MIME {
    pub fn parse(raw_content_type: &[u8]) -> Result<MIME, Error> {
        let content_type = match str::from_utf8(raw_content_type) {
            Ok(x) => x,
            _ => return Err(Error::Illformed),
        }
        .trim_start()
        .trim_end();
        Self::str_parse(&content_type)
    }

    pub fn str_parse(content_type: &str) -> Result<Self, Error> {
        debug!("Content type is: {}", content_type);
        // main_type.len() + '/' + sub_type.len() + '+' +
        let (main_type, right) = match content_type.find('/') {
            Some(pos) => (0usize..pos, pos..content_type.len()),
            _ => return Err(Error::Illformed),
        };

        if !is_restricted_name(&content_type[main_type.clone()]) {
            return Err(Error::InvaliMainType);
        }

        if !content_type[right.clone()].chars().any(is_restricted_char) {
            return Err(Error::Illformed);
        }

        let sub_end = content_type[right.clone()].find(';').unwrap_or(right.len()) + main_type.end;
        let sub_type = main_type.end + 1..sub_end;

        let mut vec = Vec::new();
        let _right_part = &content_type[sub_end..];
        let mut offset = sub_end;
        let mut mutants = Vec::new();
        let mut immortal = false;
        while let Some((name_range, value_range, new_offset)) = parse_param(content_type, offset)? {
            match name_range {
                ParamType::Mutant(name_range) => {
                    vec.push((name_range, value_range.clone()));
                    let value = &content_type[value_range];
                    for mutant_id in value.split(',') {
                        // hexed mutant id doesn't have a prefix '0x'
                        let mutant_id_hex = mutant_id.trim_matches(is_ows);
                        if mutant_id_hex.len() != 64 {
                            return Err(Error::MutantIDNotValid);
                        }
                        let mutant_id_c_str =
                            CString::new(mutant_id_hex).map_err(|_| Error::MutantIDNotValid)?;
                        let mutant_id: [u8; 32] = decode_hex(mutant_id_c_str.as_c_str())
                            .map_err(|_| Error::MutantIDNotValid)?
                            .try_into()
                            .unwrap();
                        if mutants.contains(&mutant_id) {
                            return Err(Error::DuplicateMutantId);
                        }
                        mutants.push(mutant_id);
                    }
                }
                ParamType::Generic(name_range) => {
                    vec.push((name_range, value_range));
                }
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
            immortal,
        };

        Ok(mime_type)
    }

    pub fn params(&self) -> &Vec<(RangePair, RangePair)> {
        &self.params
    }

    pub fn mut_params(&mut self) -> &mut Vec<(RangePair, RangePair)> {
        &mut self.params
    }

    pub fn get_param(&self, content_type: &[u8], param: &str) -> Result<Option<RangePair>, Error> {
        for (param_range, value_range) in self.params.iter() {
            check_range_validate(content_type, param_range)?;
            if content_type[param_range.clone()] == param.as_bytes()[..] {
                check_range_validate(content_type, value_range)?;
                return Ok(Some(value_range.clone()));
            }
        }
        Ok(None)
    }
}

fn check_range_validate(array: &[u8], range: &Range<usize>) -> Result<(), Error> {
    let end: usize = range.end;
    debug!("len = {}, end = {end}", array.len());
    if array.len() < end {
        return Err(Error::ContentOutOfRange);
    }
    Ok(())
}

fn is_restricted_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '*') && is_restricted_str(s)
}

fn is_restricted_name_patched(s: &str) -> bool {
    s == "mutant[]" || is_restricted_name(s)
}

fn is_restricted_value_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+' | '%' | '*' | '\'' | ','
        )
}

fn is_restricted_str(s: &str) -> bool {
    s.chars().all(is_restricted_char)
}

fn is_restricted_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+' | '%' | '*' | '\''
        )
}

const fn is_ows(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn parse_param(
    source: &str,
    offset: usize,
) -> Result<Option<(ParamType, RangePair, usize)>, Error> {
    if offset >= source.len() {
        return Ok(None);
    }
    let s = &source[offset..];
    let (lhs, rhs) = match s.split_once(';') {
        Some((lhs, rhs)) if lhs.chars().all(is_ows) && rhs.chars().all(is_ows) => return Ok(None),
        Some((lhs, rhs)) if lhs.chars().all(is_ows) => (lhs, rhs),
        _ if s.chars().all(is_ows) => return Ok(None),
        _ => return Err(Error::InvalidParams),
    };

    let (name, value) = match rhs.split_once('=') {
        Some((name, value_maybe)) => match value_maybe.split_once(';') {
            None => (name, value_maybe),
            Some((value_maybe_lhs, _)) => (name, value_maybe_lhs),
        },
        _ => return Err(Error::InvalidParams),
    };

    let key_trimmed = name.trim_start_matches(is_ows).len();
    let key_start = lhs.len() + 1 + name.len() - key_trimmed;
    let key_range = key_start + offset..key_start + offset + key_trimmed;
    if !is_restricted_name_patched(&source[key_range.clone()]) {
        return Err(Error::InvalidParams);
    }
    let key = match &source[key_range.clone()] {
        "immortal" => ParamType::Immortal(key_range.clone()),
        "mutant[]" => ParamType::Mutant(key_range.clone()),
        _ => ParamType::Generic(key_range.clone()),
    };
    let value_start = key_range.end + 1;
    if let Some(value) = value.strip_prefix('\"') {
        let value_end = value_start + parse_quoted_value(value)? + 1;
        let value_range = value_start..value_end;
        Ok(Some((key.clone(), value_range.clone(), value_end)))
    } else {
        let value_end = value_start
            + value
                .chars()
                .take_while(|&c| is_restricted_value_char(c))
                .map(char::len_utf8)
                .sum::<usize>();
        let value_range = value_start..value_end;
        Ok(Some((key.clone(), value_range.clone(), value_end)))
    }
}

fn parse_quoted_value(s: &str) -> Result<usize, Error> {
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
            '\n' => return Err(Error::InvalidParamValue),
            _ => (),
        }
    }
    Err(Error::InvalidParamValue)
}
