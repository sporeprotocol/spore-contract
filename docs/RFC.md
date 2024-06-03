# RFC: Spore Protocol Spec

This RFC proposes the Spore Protocol (Spore for short) specification.

The core concept of Spore is anchored around the following functionalities:

- Immutable content issuance
- Permanent on-chain storage
- Simplified cell structure, devoid of noise
- Built-in extensibility in protocol design

There are three cell types defined in this protocol: `Spore Cell`, `Spore Cluster Cell`.

The Spore Protocol necessitates only a single cell type referred to as Spore Cell. Any other cell types are optional or are treated as extension cells, intended to augment to information of a Spore item.


## Data Structure

### Spore Cell

```yaml
data:
    content-type: Bytes # String Bytes
    content: Bytes
    # OPTIONAL
    cluster_id: Bytes
type:
    hash_type: "data1"
    code_hash: SPORE_TYPE_DATA_HASH
    args: SPORE_ID
lock:
    <user_defined>
```

- `content-type` hint text data of the formats in the `content` field, also can holds extension feature labels like `TYPE/SUBTYPE;PARAM=VAL` . It should follow the [standard of MIME](https://datatracker.ietf.org/doc/html/rfc2046). For example, `image/png` indicates this Spore contains a PNG image. While users can use this param to extend the protocol, there is preset of params provided by default:
    - `immortal` is a param defines whether this NFT is undestructible or not, default is `false`. for example: `content-type: image/png;immortal=true`
- `content` This field contains the main content of the NFT.
- `cluster_id` An optional field used to denote the series or class collection of this Spore NFT item. Refer to the [Spore Cluster Cell](https://www.notion.so/Spore-NFT-Draft-Spec-old-27e391dc259f4c4bad924d1a2fc26dfc?pvs=21) section for more details.
- `type` script is set to `SPORE_TYPE_DATA_HASH`  with args equals to `SPORE_ID`, which follows: `SPORE_ID = hash(this_transaction.inputs[0] | output_index_of_this_cell)`.

All the fields in a `Spore Cell` are immutable once created.

### Spore Cluster Cell

The structure of a Cluster Cell in Spore Protocol is defined as follows:

```yaml
data:
    name: Bytes # String Bytes
    description: Bytes # String Bytes
type:
    hash_type: "data1"
    code_hash: CLUSTER_TYPE_DATA_HASH
    args: CLUSTER_ID
lock:
    <user_defined>
```

- `name` Represents the name of the Spore Cluster.
- `description` Provides a textual description of this Cluster.
- `type` script is set to `CLUSTER_TYPE_DATA_HASH` with args equals to `CLUSTER_ID` , which follows the rules of Type ID script. And we define the `CLUSTER_ID = hash(this_transaction.inputs[0] | output_index_of_this_cell)`.

A `Spore Cluster Cell` is *indestructible*  and immutable once created.

## Examples

### Single Spore Issuance/Minting

Below is a sample transaction for creating a Spore contains PNG image.

```yaml
CellDep:
  <Spore Type Cell>
  <...>
Inputs:
  <any normal ckb cells>
Outputs:
  Spore Cell:
    Capacity: N CKBytes
    Type:
      hash_type: "data1"
      code_hash: SPORE_TYPE_DATA_HASH
      args: SPORE_ID
    Lock:
      <user-defined>
    Data:
      content-type: "image/png"
      content: BYTES_OF_THE_IMAGE
      cluster_id: null
Witnesses:
  <valid signature for public key hash>
```

If `cluster_id` was set, then the referenced `Cluster Cell` should appear in `CellDep`, and should follow rules below:

- Rule 1: The referenced Cluster Cell **must** exist in CellDep.
- Rule 2: The referenced Cluster Cell should be exist in both Inputs and Outputs. If not, Rule 5 should be applied.
- Rule 3: The input Cluster Cell should have a lock script that is unlock-able
- Rule 4: Cluster Cell with same Type Script Args in Outputs should have a same lock pair with in Inputs.
- Rule 5: If Rule 2~4 is not fit, at least one cell with a same lock of the referenced Cluster Cell should be exist in both Inputs and Outputs. We call these cell as “Lock Proxy Cell”. A Spore Cell can also be Lock Proxy Cell.

Below is an example showing the transaction when `cluster_id` is set using Cluster as Inputs:

```yaml
CellDep:
  <Spore Type Script Cell>
  <Cluster Type Script Cell>
  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <other deps...>
Inputs:
  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <other deps...>
  <other ckb cells...>
Outputs:
  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <other deps...>
  Spore Cell:
    Type:
      hash_type: "data1"
      code_hash: SPORE_V1_DATA_HASH # hash of Spore's type script data hash
		  args: TYPE_ID
    Lock:
      <user-defined>
    Data:
      content-type: "image/png"
      content:  BYTES_OF_THE_IMAGE
      cluster_id: "0xbfca51165"
  <other ckb cells...>
```

Below is an example showing the transaction when `cluster_id` is set using Lock Proxy Cells:

```yaml
CellDep:
  <Spore Type Script Cell>
  <Cluster Type Script Cell>
  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <other deps...>
Inputs:
  Cell 1: # Lock Proxy Cell
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
  <other ckb cells...>
Outputs:
  Cell 2: # Lock Proxy Cell
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
  Spore Cell:
    Type:
      hash_type: "data1"
      code_hash: SPORE_V1_DATA_HASH # hash of Spore's type script data hash
		  args: TYPE_ID
    Lock:
      <user-defined>
    Data:
      content-type: "image/png"
      content:  BYTES_OF_THE_IMAGE
      cluster_id: "0xbfca51165"
  <other ckb cells...>
```

### Cluster Creation

Below is a sample transaction for creating a `Cluster Cell`:

```yaml
Inputs:
  <Cluster Type Script Cell>
  <...>
Outputs:
  Spore Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: CLUSTER_ID
    Lock:
      <user-defined>
    Data:
      name: "NAME_OF_CLUSTER"
      description: "THIS IS A DESCR FOR THIS CLUSTER"
```

### Multiple Spore Issuance/Minting

Below is a sample transaction for creating several Spore in one operation

```yaml
CellDep:
  <Spore Type Script Cell>
  <Cluster Type Script Cell>
  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH
      args: LOCK_ARGS
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <other deps...>
Inputs:
  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH
      args: LOCK_ARGS
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <any other normal ckb cells...>
Outputs:
  Spore Cell1:
    Capacity: N CKBytes
    Type:
      hash_type: "data1"
      code_hash: SPORE_TYPE_DATA_HASH
      args: SPORE_ID_1
    Lock:
      <user-defined>
    Data:
      content-type: "image/png"
      content: BYTES_OF_THE_IMAGE1
      cluster_id: 0xbfca51165

  Spore Cell2:
    Capacity: N CKBytes
    Type:
      hash_type: "data1"
      code_hash: SPORE_TYPE_DATA_HASH
      args: SPORE_ID_2
    Lock:
      <user-defined>
    Data:
      content-type: "image/png"
      content: BYTES_OF_THE_IMAGE2
      cluster_id: null

  Spore Cell3:
    Capacity: N CKBytes
    Type:
      hash_type: "data1"
      code_hash: SPORE_TYPE_DATA_HASH
      args: SPORE_ID_3
    Lock:
      <user-defined>
    Data:
      content-type: "plain/text"
      content: BYTES_OF_THE_TEXT
      cluster_id: 0xbfca51165

  Cluster Cell:
    Type:
      hash_type: "data1"
      code_hash: CLUSTER_TYPE_DATA_HASH
      args: 0xbfca51165
    Lock:
      code_hash: LOCK_HASH
      args: LOCK_ARGS
    Data:
      name: CLUSTER_NAME
      description: DESCRIPTION
  <...>
Witnesses:
  <valid signature for inputs>
```

### Transfer

Below is an example transaction transfers Spore from one to other holder
```yaml
Inputs:
  Spore Cell:
    Capacity: N CKBytes
    Data:
      content-type: "image/png"
      content: BYTES_OF_THE_IMAGE
    Type:
      hash_type: "data1"
      code_hash: SPORE_TYPE_DATA_HASH
      args: SPORE_ID
    Lock:
      code_hash: LOCK_HASH_1
      args: LOCK_ARGS_1
  <...>
Outputs:
  Spore Cell:
    Capacity: N CKBytes
    Data:
      content-type: "image/png"
      content: BYTES_OF_THE_IMAGE
    Type:
      hash_type: "data1"
      code_hash: SPORE_TYPE_DATA_HASH
      args: SPORE_ID
    Lock:
      code_hash: LOCK_HASH_2
      args: LOCK_ARGS_2
  <...>
Witnesses:
  <valid signature for inputs>
```