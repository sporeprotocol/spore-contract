# Public Cluster Verification Problems

## Original Problems

In the original design of the Spore Protocol, the creation of a Spore with a Cluster ID required the following conditions:

1. The referenced Cluster Cell must be found in the `CellDeps`.
2. The referenced Cluster Cell must be found in the `Inputs`.
3. The referenced Cluster Cell must be found in the `Outputs`.

This verification process ensures ownership of the referenced Cluster Cell, preventing malicious activities on one's private Cluster.

While this design prevents minting in a private Cluster when ownership is absent, challenges remain in public Clusters. In public mining, ownership is nullified upon a customized lock (e.g., [anyone-can-pay](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0026-anyone-can-pay/0026-anyone-can-pay.md)) is introduced. However, this gives rise to the issue of cell competition: if two minting transactions are accidentally sent simultaneously, one will be rejected. The target Cluster gets consumed and recreated by one transaction, requiring reconstruction of the rejected transaction to proceed with minting.

## Solution

At this stage, the **owner** needs to create a special cell, here we called it **Cluster Proxy Cell**

A Cluster Proxy Cell’s structure is like below:

```yaml
Cluster Proxy Cell:
    Data: REFERENCED_CLUSTER_ID
    Type:
        code_hash: CLUSTER_PROXY_TYPE_HASH
        args: <cluster_proxy_id> [<min_payment>]
    Lock:
        <user_defined>
```

The Type args can be:

- args: <cluster_proxy_id>
- args: <cluster_proxy_id> <minimal payment in 10^n ckbytes: uint8>

Where `cluster_proxy_id = hash(Inputs[0], Output_Index)`

### Step1: Creating Cluster Proxy Cell

Creating a Cluster Proxy Cell can be done in two ways. The first method is putting a Cluster Cell to Inputs & Outputs, as shown below:

#### Method 1. Use Direct Input

```yaml
CellDeps:
    <CLUSTER_PROXY_TYPE_CELL>
    <CLUSTER_TYPE_CELL>
    Cluster Cell A:
        Data: <...>
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_TYPE_HASH_A
        Lock: <user-defined>
Inputs:
    Cluster Cell A:
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_TYPE_HASH_A
        Lock: <user-defined>
    <...any other cells>
Outputs:
    Cluster Cell A:
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_TYPE_HASH_A
        Lock: <user-defined>
    Cluster Proxy Cell:
        Data: CLUSTER_ID_A
        Type:
            code_hash: CLUSTER_PROXY_TYPE_HASH
            args: CLUSTER_PROXY_ID_A
        Lock:
            <user_defined> # for example, acp
```

The other method is using an Input Cell with same Lock to the Cluster Cell to create a Cluster Proxy Cell.

#### Method 2. Use Lock Proxy

```yaml
CellDeps:
    <CLUSTER_PROXY_TYPE_CELL>
    Cluster Cell A:
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_TYPE_HASH_A
        Lock:
            args: <public key hash A>
            code_hash: LOCK_CODE_HASH_A
Inputs:
    Any Cell:
        Lock:
            args: <public key hash A>
            code_hash: LOCK_CODE_HASH_A
    <...any other cells>
Outputs:
    Cluster Proxy Cell:
        Data: CLUSTER_ID_A
        Type:
            code_hash: CLUSTER_PROXY_TYPE_HASH
            args: CLUSTER_PROXY_ID
        Lock:
            <user_defined> # for example, acp
     <...any other cells>
```

### Step2: Create Cluster Agent Cell

Once the Cluster owner created the Cluster Proxy Cell, anyone who is able to unlock the Cluster Proxy Cell is able to create a special type cell called: Cluster Agent Cell. Holder of this Cluster Agent Cell can mint Spore in a regular process and put it into the referenced Cluster.

```yaml
Cluster Agent Cell:
    Data: Type Hash of Referenced Cluster Proxy
    Type:
        code_hash: CLUSTER_AGENT_TYPE_HASH
        args: REFERENCED_CLUSTER_ID
    Lock:
        <user_defined>
```

There are two ways to create a Cluster Proxy Agent Cell.

#### Method 1. Direct Input

```yaml
CellDeps:
    <CLUSTER_PROXY_TYPE_CELL>
    <CLUSTER_PROXY_AGENT_TYPE_CELL>
    Cluster Proxy Cell:
        Data: CLUSTER_ID_A
        Type:
            code_hash: CLUSTER_PROXY_TYPE_HASH
            args: CLUSTER_PROXY_ID
        Lock:
            <user_defined>
Inputs:
    Cluster Proxy Cell:
        Data: CLUSTER_ID_A
        Type:
            code_hash: CLUSTER_PROXY_TYPE_HASH
            args: CLUSTER_PROXY_ID
        Lock:
            <user_defined>
    <...any other cells>
Outputs:
     Cluster Agent Cell:
        Data: Hash(ClusterProxyCell.Type)
        Type:
            code_hash: CLUSTER_AGENT_TYPE_HASH
            args: CLUSTER_ID_A
        Lock:
            <user_defined>

    Cluster Proxy Cell:
        Data: CLUSTER_ID_A
        Type:
            code_hash: CLUSTER_PROXY_TYPE_HASH
            args: CLUSTER_PROXY_ID
        Lock:
            <user_defined>
     <...any other cells>
```

# Method 2. Use Payment Appearance

Alternatively, you can make a payment to the Cluster Proxy owner to create a Cluster Agent Cell. This method essentially involves transferring capacity to the same lock address with the Cluster Proxy.

```yaml
CellDeps:
    <CLUSTER_AGENT_TYPE_CELL>
    Cluster Proxy Cell:
        Data: CLUSTER_ID_A
        Type:
            code_hash: CLUSTER_PROXY_TYPE_HASH
             args: <CLUSTER_PROXY_ID_A> <MINIMAY_PAYMENT_A>
        Lock:
            code_hash: CODE_HASH_A
            args: PUBKEY_A
Inputs:
    Payment Cell: #
        Capacity: N # N >= MINIMAY_PAYMENT_A
        Lock: <user_defined>
    <...any other cells>
Outputs:
     Cluster Agent Cell:
        Data: Hash(ClusterProxyCell.Type)
        Type:
            code_hash: CLUSTER_AGENT_TYPE_HASH
            args: CLUSTER_ID_A
        Lock:
            <user_defined>

    Receivement Cell:
        Capacity: N
        Lock:
            code_hash: CODE_HASH_A
            args: PUBKEY_A
     <...any other cells>
```

Here, the payment cell serves merely as an example; it can be any unlockable cell and is not limited to only one cell.

### Step3: Mint Spore with Cluster Agent

The Cluster Agent Cell holder can mint Spore using three valid methods listed below.

#### Method 1. Mint With Direct Input

```yaml
CellDeps:
    <SPORE_TYPE>
Inputs:
    Cluster Agent Cell A:
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_AGENT_TYPE_HASH
        Lock: <user-defined>
    <...any other cells>
Outputs:
    Spore Cell:
        Capacity: N CKBytes
        Data:
            content-type: "image/png"
            content:  BYTES_OF_THE_IMAGE
            cluster: CLUSTER_ID_A
        Type:
            hash_type: "data1"
   			code_hash: SPORE_TYPE_DATA_HASH # hash of Spore's type script data hash
   			args: SPORE_ID
        Lock:
            <user-defined>
    Cluster Agent Cell A:
        Data: Hash(ClusterProxyCell.Type)
        Type:
            code_hash: CLUSTER_AGENT_TYPE_HASH
            args: CLUSTER_ID_A
        Lock:
            <user_defined> # for example, acp
```

#### Method 2. Mint With Lock Proxy

```yaml
CellDeps:
    <SPORE_TYPE_CELL>
    Cluster Agent Cell A:
        Data: Hash(Cluster_Proxy_Cell_Type)
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_AGENT_TYPE_HASH
        Lock: 
            args: <public key hash A>
            code_hash: LOCK_CODE_HASH_A
Inputs:
     Any Cell: # Lock Proxy Cell
        Lock: 
            args: <public key hash A>
            code_hash: LOCK_CODE_HASH_A
     <...any other cells>
Outputs:
    Spore Cell:
        Capacity: N CKBytes
        Data:
            content-type: "image/png"
            content:  BYTES_OF_THE_IMAGE
            cluster: CLUSTER_ID_A
        Type:
            hash_type: "data1"
            code_hash: SPORE_TYPE_DATA_HASH # hash of Spore's type script data hash
            args: SPORE_ID
        Lock:
            <user-defined>
```

#### Method 3. Mint With Signature (Not Implemented)

```yaml
CellDeps:
    <SPORE_TYPE_CELL>
    Cluster Agent Cell A:
        Type:
            args: CLUSTER_ID_A
            code_hash: CLUSTER_AGENT_TYPE_HASH
        Lock: 
            args: <public key hash A>
            code_hash: LOCK_CODE_HASH_A
Inputs:
     <...any other cells>
Outputs:
    Spore Cell:
        Capacity: N CKBytes
        Data:
            content-type: "image/png"
            content:  BYTES_OF_THE_IMAGE
            cluster: CLUSTER_ID_A
        Type:
            hash_type: "data1"
            code_hash: SPORE_V1_DATA_HASH # hash of Spore's type script data hash
            args: SPORE_ID
        Lock:
            <user-defined>
Witnesses:
    <valid signature for public key hash A>
```
