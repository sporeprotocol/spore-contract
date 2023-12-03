# Mutant
Mutant is a special extension cell in Spore eco. It stores Lua scripting code inside the data field, and will be executed & evaluated in every referenced transaction once applied. One can apply mutants to:

1. A Spore by specifying in `content-type` using param `mutant[] = MUTANT_ID_1, MUTANT_ID2,…;`, and including the referenced mutant cell in `CellDep`. You'll also need to include Spore Lua Lib binary in CellDeps.
2. A cluster by setting cluster’s `mutant_id=MUTANT_ID`, and including the referenced mutant cell in `CellDep`.

For example, you can write:
```Lua
print("Hello, world!");
if spore_output_index > 0 then
  ckb.exit_script(spore_output_index)
end
```
Storing it on a Mutant Cell. Assume the ID of this mutant is `0xMUTANT_ID` we have a spore like:
```yaml
Data:
  content-type: "image/png;mutant[]=0xMUTANT_ID"
  content: ...
Type:
  code_hash: SPORE_TYPE_ID_V2
  args: 0xSPORE_ID
```
Which will result:
1. If the spore exist in `Output[0]`, it will output `"Hello, world!"` in every execution and return success(code 0);
2. If the spore exist in `Output[1]` (any index > 0), it will exit with a faliure code `1` (equals to Output Index as we defined in the Lua code)

Check [Lua docs](https://www.lua.org/docs.html) for standard Lua language doc, and [CKB Lua Functions](https://github.com/nervosnetwork/ckb-lua-vm/blob/master/docs/dylib.md#lua-functions) for CKB related Lua APIs


## OP Code
Mutant has three execution modes mapped to three types of opcode, and will be automatically detected during transaction:
- opcode `0`: Spore in minting operation, mutant executed as minting mode, and mutant Lua script can use external values: `spore_ext_mode`, `spore_output_index`, and `spore_ext_mode = 0`
- opcode `1`: Spore in transfer operation, mutant executed as transfer mode, and mutant Lua script can use external values: `spore_ext_mode`, `spore_input_index` `spore_output_index`, and `spore_ext_mode = 1`
- opcode `2`: Spore in melt operation, mutant executed as melt mode, and mutant Lua script can use external values: `spore_ext_mode`,`spore_input_index`, and `spore_ext_mode = 2`

## Deployment

### Pudge Testnet
#### Spore Lua Lib
- code_hash: `0xed08faee8c29b7a7c29bd9d495b4b93cc207bd70ca93f7b356f39c677e7ab0fc`
- tx: `0xa3add6709887b3e136546edb024cd905726d73a126d47764b4537e8b08de390f`
- index: `0`

#### Mutant
- code_hash: `0xf979ff194202dd2178c18cfc2e5cc60c965a1c94aad8a46eb80e74ee85842b5ce`
- tx: `0xa3add6709887b3e136546edb024cd905726d73a126d47764b4537e8b08de390f`
- index: `1`


## RFC
### Data Structure
```yaml
Data:
  LUA_CODE_DATA_BYTES
Type:
  hash_type: "data1"
  code_hash: SPORE_MUTANT_TYPE_HASH
  args: <MUTANT_ID>[<MINIMAL_PAYMENT>]
Lock: <user_defined>
```
Available Mutant args are list as below:
```yaml
<32bytes Mutant ID>
<32bytes Mutant ID><1bytes CKByte minimum>
```
Where `Mutant ID = hash(Inputs[0], Output_Index)`. The value stored in CKByte minimum  amount are interpreted in the following way: 

if `x` is stored in the field, the minimal transfer amount will be `10^x`, for example:

- If 3 is stored in CKByte minimum, it means the minimal payment amount to use this mutant cell is 1000 shannons
- If 0 is stored in CKByte minimum, it means the minimal payment amount to use this mutant cell is 1 shannon

The additions of CKByte minimums enforce a minimal payment for one to reference this mutant extension while minting Spore.

When applying a Mutant Extension to a Spore, it will cause:

1. contract will run extension code using `ckb_std::exec`
2. arguments of `exec` will be packed as:
argv[0]: `content-type` of Spore
argv[1]: Type `args` (Spore ID) of Spore
argv[2]: Spore `content`
3. Result of exec will be performed:
`0` : success, this operation to Spore is valid and will continue to finish;
any other codes: failed. operation will abort, transaction will return failure code

These effects will be performed once every time during Spore’s creation, transfer, and destruction.
