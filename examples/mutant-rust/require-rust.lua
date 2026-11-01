-- Replace @CODE_HASH@ with the deployed mutant_equipment binary's data hash.
-- Mode 0 creates the Mutant itself; 1 mints a Spore; 2/3 transfer/melt it.
if spore_ext_mode ~= 1 then return end

local code_hash = ("@CODE_HASH@"):gsub("..", function(hex)
  return string.char(tonumber(hex, 16))
end)
local target, err = ckb.load_cell_by_field(
  spore_output_index, ckb.SOURCE_OUTPUT, ckb.CELL_FIELD_TYPE_HASH)
assert(err == nil and #target == 32)

-- Exact Molecule Script: Data1, companion code hash, target script hash args.
-- A cell dep or an input is insufficient: CKB must execute this output's type.
local expected = string.pack("<I4I4I4I4", 85, 16, 48, 49)
  .. code_hash .. string.char(2) .. string.pack("<I4", 32) .. target
for index = 0, 255 do
  local script, load_err = ckb.load_cell_by_field(
    index, ckb.SOURCE_OUTPUT, ckb.CELL_FIELD_TYPE)
  if load_err == nil and script == expected then return end
end
ckb.exit_script(-1)
