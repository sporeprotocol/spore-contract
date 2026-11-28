-- Equivalent Lua-only recipe, used for compatibility and cycle comparison.
if spore_ext_mode ~= 1 then return end
local data, err = ckb.load_cell_data(spore_output_index, ckb.SOURCE_OUTPUT)
assert(err == nil and #data <= 4096)
-- Spore has already parsed the Molecule table before invoking its Mutant.
local offset = string.unpack("<I4", data, 9)
local length = string.unpack("<I4", data, offset + 1)
if length ~= 3 then ckb.exit_script(-1) end
local a, b, result = string.byte(data, offset + 5, offset + 7)
if a == 0 or b == 0 or a + b > 255 or a + b ~= result then
  ckb.exit_script(-1)
end
