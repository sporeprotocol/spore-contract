use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_types::{bytes::Bytes, packed::*, prelude::*};
use ckb_testtool::context::Context;

pub fn build_always_success_script(context: &mut Context, args: Bytes) -> Script {
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // build lock script
    context
        .build_script(&always_success_out_point, args)
        .expect("always success script")
}

pub fn build_output(
    context: &mut Context,
    capacity: u64,
    type_script: Option<Script>,
    lock_args: Bytes,
) -> CellOutput {
    let lock_script = build_always_success_script(context, lock_args);
    CellOutput::new_builder()
        .capacity(capacity.pack())
        .lock(lock_script)
        .type_(ScriptOpt::new_builder().set(type_script).build())
        .build()
}

pub fn build_outpoint(
    context: &mut Context,
    capacity: u64,
    type_script: Option<Script>,
    data: Bytes,
    lock_args: Bytes,
) -> OutPoint {
    let output = build_output(context, capacity, type_script, lock_args);
    context.create_cell(output, data)
}

pub fn build_input(
    context: &mut Context,
    capacity: u64,
    type_script: Option<Script>,
    data: Bytes,
    lock_args: Bytes,
) -> CellInput {
    let outpoint = build_outpoint(context, capacity, type_script, data, lock_args);
    CellInput::new_builder()
        .since(Uint64::default())
        .previous_output(outpoint)
        .build()
}
