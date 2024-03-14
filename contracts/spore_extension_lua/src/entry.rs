// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

use alloc::borrow::ToOwned;
// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::ffi::CString;
use alloc::string::String;
use alloc::{format, vec, vec::Vec};

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::debug;
use ckb_std::dynamic_loading_c_impl::{CKBDLContext, Library, Symbol};
use ckb_std::env::Arg;
use ckb_std::high_level::{load_cell_data, load_cell_type, QueryIter};
use core::ffi::{c_char, c_int, c_ulong, c_void};
use spore_errors::error::Error;
use spore_utils::{find_position_by_type, verify_type_id};

use crate::error::WrappedError;
use crate::hash::CKB_LUA_LIB_CODE_HASH;

type CreateLuaInstanceType = unsafe extern "C" fn(c_ulong, c_ulong) -> *mut c_void;
type EvaluateLuaInstanceType = unsafe extern "C" fn(
    instance: *mut c_void,
    code: *const c_char,
    code_size: usize,
    name: *const c_char,
) -> c_int;

const SPORE_EXT_NORMAL_ARG_LEN: usize = 32;
const SPORE_EXT_MINIMAL_PAYMENT_ARG_LEN: usize = 32 + 8; // 32 bytes hash + u64 payment

struct CKBLuaLib {
    lib: Library,
}

impl CKBLuaLib {
    pub fn new() -> Result<Self, Error> {
        debug!("prepare lua lib");
        let mut context = unsafe { CKBDLContext::<[u8; 270 * 1024]>::new() };
        #[allow(deprecated)]
        let lib = context
            .load(&CKB_LUA_LIB_CODE_HASH)
            .map_err(|_| Error::FailedToLoadLuaLib)?;
        Ok(Self { lib })
    }

    pub fn evaluate_lua_script(
        &self,
        index: usize,
        prefix_code: Option<String>,
    ) -> Result<(), WrappedError> {
        let mut code_base = load_cell_data(index, Output)?;
        if let Some(prefix_code) = prefix_code {
            let mut prefix_code = prefix_code.as_bytes().to_vec();
            prefix_code.append(&mut code_base);
            code_base = prefix_code;
        }
        self.execute_lua_script(&code_base)?;
        Ok(())
    }

    fn create_lua_instance(&self) -> Result<*mut c_void, Error> {
        match unsafe { self.lib.get(b"lua_create_instance") } {
            Some(create_lua_instance) => {
                let mut lua_mem = vec![0u8; 500 * 1024];
                unsafe {
                    let instance = (create_lua_instance as Symbol<CreateLuaInstanceType>)(
                        lua_mem.as_mut_ptr() as c_ulong,
                        lua_mem.as_mut_ptr().offset(500 * 1024) as c_ulong,
                    );
                    if instance.is_null() {
                        return Err(Error::FailedToCreateLuaInstance);
                    }
                    Ok(instance)
                }
            }
            None => {
                // not a valid lua lib, maybe error deployment
                Err(Error::InvalidLuaLib)
            }
        }
    }

    pub fn execute_lua_script(&self, code: &Vec<u8>) -> Result<(), WrappedError> {
        let instance = self.create_lua_instance()?;
        let ret = match unsafe { self.lib.get(b"lua_run_code") } {
            Some(lua_run_code) => {
                let size = code.len().clone();
                let ret = unsafe {
                    (lua_run_code as Symbol<EvaluateLuaInstanceType>)(
                        instance,
                        code.as_ptr() as *const i8,
                        size,
                        CString::new("SporeExtension").unwrap_or_default().as_ptr(),
                    )
                };
                Ok(ret as i8)
            }
            None => Err(Error::InvalidLuaLib),
        }?;

        if ret == 0 {
            return Ok(());
        }
        // we recommend the error code follows this pattern: -127 <= ret <= -1 or 100 <= ret <= 127
        else if 0 < ret && ret < Error::Unknown as i8 {
            return Err(Error::InvalidLuaScript.into());
        } else {
            return Err(WrappedError::LuaError(ret));
        }
    }
}

fn process_creation(index: usize) -> Result<(), WrappedError> {
    if verify_type_id(index).is_none() {
        return Err(Error::InvalidExtensionID.into());
    }
    let args = load_cell_type(index, Output)?
        .unwrap_or_default()
        .args()
        .raw_data();
    match args.len() {
        SPORE_EXT_NORMAL_ARG_LEN | SPORE_EXT_MINIMAL_PAYMENT_ARG_LEN => {}
        _ => {
            return Err(Error::InvalidExtensionArg.into());
        }
    }
    let lua_lib = CKBLuaLib::new()?;

    let prefix_code = "local spore_ext_mode = 0\n".to_owned();
    lua_lib.evaluate_lua_script(index, Some(prefix_code))?;
    Ok(())
}

fn process_transfer() -> Result<(), WrappedError> {
    let input_data = load_cell_data(0, GroupInput)?;
    let output_data = load_cell_data(0, GroupOutput)?;

    if input_data != output_data {
        return Err(Error::ModifyExtensionPermanentField.into());
    }

    Ok(())
}

fn execute_code_create(extension_index: usize, target_index: usize) -> Result<(), WrappedError> {
    let mut code_base =
        format!("local spore_ext_mode = 1\nlocal spore_output_index = {target_index}\n")
            .as_bytes()
            .to_vec();
    let mut ext_code = load_cell_data(extension_index, CellDep)?;
    code_base.append(&mut ext_code);
    let lua_lib = CKBLuaLib::new()?;
    lua_lib.execute_lua_script(&code_base)
}

fn execute_code_transfer(
    extension_index: usize,
    input_index: usize,
    output_index: usize,
) -> Result<(), WrappedError> {
    let mut code_base = format!(
        "local spore_ext_mode = 2\nlocal spore_input_index = {input_index}\nlocal spore_output_index = {output_index}\n"
    )
    .as_bytes()
    .to_vec();
    let mut ext_code = load_cell_data(extension_index, CellDep)?;
    code_base.append(&mut ext_code);
    let lua_lib = CKBLuaLib::new()?;
    lua_lib.execute_lua_script(&code_base)
}

fn execute_code_destroy(extension_index: usize, input_index: usize) -> Result<(), WrappedError> {
    let mut code_base =
        format!("local spore_ext_mode = 3\nlocal spore_input_index = {input_index}\n")
            .as_bytes()
            .to_vec();
    let mut ext_code = load_cell_data(extension_index, CellDep)?;
    code_base.append(&mut ext_code);
    let lua_lib = CKBLuaLib::new()?;
    lua_lib.execute_lua_script(&code_base)
}

pub fn main(argv: &[Arg]) -> Result<(), WrappedError> {
    if argv.is_empty() {
        debug!("running internally");
        // creation/transfer mode
        let extension_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
            .map(|script| script.unwrap_or_default())
            .collect();

        if extension_in_output.len() > 1 {
            return Err(Error::ConflictCreation.into());
        }

        let extension_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
            .map(|script| script.unwrap_or_default())
            .collect();

        if extension_in_input.len() > 1 {
            return Err(Error::MultipleSpend.into());
        }

        return match (extension_in_input.len(), extension_in_output.len()) {
            (0, 1) => {
                // find it's index in Source::Output
                let output_index =
                    find_position_by_type(&extension_in_output[0], Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
                process_creation(output_index)
            }
            (1, 1) => {
                return process_transfer();
            }
            _ => Err(Error::InvalidExtensionOperation.into()), // Can not destroy a extension cell(for safety)
        };
    } else {
        // execution mode
        debug!("running externally");
        match argv[0].to_bytes() {
            &[48] => {
                // 0, CREATE SPORE
                debug!("Spore Creation with extension!");
                let spore_extension_index = argv[1]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                let target_index = argv[2]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                execute_code_create(spore_extension_index, target_index)?;
            }
            &[49] => {
                // 1, TRANSFER SPORE
                debug!("Spore Transfer with extension!");
                let spore_extension_index = argv[1]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                let input_index = argv[2]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                let output_index = argv[3]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                execute_code_transfer(spore_extension_index, input_index, output_index)?;
            }
            &[50] => {
                // 2, DESTROY SPORE
                debug!("Spore Destroy with extension!");
                let spore_extension_index = argv[1]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                let input_index = argv[2]
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| Error::InvalidLuaParameters)?;
                execute_code_destroy(spore_extension_index, input_index)?;
            }
            _ => return Err(Error::InvalidExtensionOperation.into()),
        }
        Ok(())
    }
}
