//! Versioned invocation ABI and returning execution for Rust-authored Mutants.
#![no_std]

extern crate alloc;

use alloc::{ffi::CString, format, vec::Vec};
use ckb_std::{ckb_constants::Source, syscalls};
use core::ffi::CStr;

/// Script hash_type=data2 selects CKB-VM V2 (including on the legacy SDK).
pub const VM2_HASH_TYPE: u8 = 4;

pub const ABI: &str = "spore-mutant-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Create,
    Mint,
    Transfer,
    Burn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub operation: Operation,
    /// Output index for Create, cell-dependency index for other operations.
    pub mutant_index: usize,
    pub input_index: Option<usize>,
    pub output_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Arguments,
    Spawn,
    Wait,
    Rejected(i8),
}

fn index(bytes: &[u8]) -> Result<usize, Error> {
    if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == b'0') {
        return Err(Error::Arguments);
    }
    bytes.iter().try_fold(0usize, |n, b| {
        if !b.is_ascii_digit() {
            return Err(Error::Arguments);
        }
        n.checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or(Error::Arguments)
    })
}

impl Invocation {
    /// Parse exactly five arguments; missing indices are encoded as `-`.
    /// Pass `ckb_std::env::argv().iter().map(|a| a.to_bytes())` collected as a slice.
    pub fn parse(args: &[&[u8]]) -> Result<Self, Error> {
        if args.len() != 5 || args[0] != ABI.as_bytes() {
            return Err(Error::Arguments);
        }
        let operation = match args[1] {
            b"create" => Operation::Create,
            b"mint" => Operation::Mint,
            b"transfer" => Operation::Transfer,
            b"burn" => Operation::Burn,
            _ => return Err(Error::Arguments),
        };
        let optional = |bytes| {
            if bytes == b"-" {
                Ok(None)
            } else {
                index(bytes).map(Some)
            }
        };
        let invocation = Self {
            operation,
            mutant_index: index(args[2])?,
            input_index: optional(args[3])?,
            output_index: optional(args[4])?,
        };
        match (operation, invocation.input_index, invocation.output_index) {
            (Operation::Create, None, None)
            | (Operation::Mint, None, Some(_))
            | (Operation::Transfer, Some(_), Some(_))
            | (Operation::Burn, Some(_), None) => Ok(invocation),
            _ => Err(Error::Arguments),
        }
    }

    pub fn execute(self) -> Result<(), Error> {
        let operation = match self.operation {
            Operation::Create => "create",
            Operation::Mint => "mint",
            Operation::Transfer => "transfer",
            Operation::Burn => "burn",
        };
        let optional = |i: Option<usize>| i.map(|n| format!("{n}")).unwrap_or_else(|| "-".into());
        let strings = [
            ABI.into(),
            operation.into(),
            format!("{}", self.mutant_index),
            optional(self.input_index),
            optional(self.output_index),
        ];
        let args: Vec<CString> = strings
            .iter()
            .map(|s: &alloc::string::String| {
                CString::new(s.as_bytes()).map_err(|_| Error::Arguments)
            })
            .collect::<Result<_, _>>()?;
        let refs: Vec<&CStr> = args.iter().map(|a| a.as_c_str()).collect();
        let source = if self.operation == Operation::Create {
            Source::Output
        } else {
            Source::CellDep
        };
        execute(self.mutant_index, source, &refs)
    }
}

fn execute(index: usize, source: Source, args: &[&CStr]) -> Result<(), Error> {
    let pointers: Vec<*const i8> = args.iter().map(|a| a.as_ptr() as *const i8).collect();
    let mut pid = 0;
    let fds = [0u64];
    let mut spawn_args = syscalls::SpawnArgs {
        argc: pointers.len() as u64,
        argv: pointers.as_ptr(),
        process_id: &mut pid,
        inherited_fds: fds.as_ptr(),
    };
    // Execute only cell data, in a fresh VM, then rejoin before continuing.
    syscalls::spawn(index, source, 0, 0, &mut spawn_args).map_err(|_| Error::Spawn)?;
    let status = syscalls::wait(pid).map_err(|_| Error::Wait)?;
    if status == 0 {
        Ok(())
    } else {
        Err(Error::Rejected(status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_checks_shapes_and_full_width_indices() {
        let call =
            Invocation::parse(&[b"spore-mutant-v1", b"transfer", b"256", b"12", b"1024"]).unwrap();
        assert_eq!(call.mutant_index, 256);
        assert_eq!(call.input_index, Some(12));
        assert_eq!(call.output_index, Some(1024));
        for args in [
            [b"spore-mutant-v2".as_slice(), b"mint", b"0", b"-", b"0"],
            [b"spore-mutant-v1".as_slice(), b"mint", b"0", b"0", b"0"],
            [b"spore-mutant-v1".as_slice(), b"burn", b"0", b"0", b"0"],
            [b"spore-mutant-v1".as_slice(), b"create", b"0", b"-", b"0"],
            [b"spore-mutant-v1".as_slice(), b"mint", b"0", b"-", b"01"],
            [
                b"spore-mutant-v1".as_slice(),
                b"mint",
                b"0",
                b"-",
                b"18446744073709551616",
            ],
            [b"spore-mutant-v1".as_slice(), b"mint", b"0", b"-", b"-1"],
        ] {
            assert_eq!(Invocation::parse(&args), Err(Error::Arguments));
        }
        assert_eq!(Invocation::parse(&[]), Err(Error::Arguments));
    }
}
