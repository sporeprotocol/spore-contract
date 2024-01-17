use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_types::{bytes::Bytes, packed::*, prelude::*};
use ckb_testtool::context::Context;

pub fn build_always_success_script(context: &mut Context) -> Script {
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // build lock script
    context
        .build_script(&always_success_out_point, Default::default())
        .expect("always success script")
}

pub fn build_output(
    context: &mut Context,
    capacity: u64,
    type_script: Option<Script>,
) -> CellOutput {
    let lock_script = build_always_success_script(context);
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
) -> OutPoint {
    let output = build_output(context, capacity, type_script);
    context.create_cell(output, data)
}

pub fn build_input(
    context: &mut Context,
    capacity: u64,
    type_script: Option<Script>,
    data: Bytes,
) -> CellInput {
    let outpoint = build_outpoint(context, capacity, type_script, data);
    CellInput::new_builder()
        .since(Uint64::default())
        .previous_output(outpoint)
        .build()
}
