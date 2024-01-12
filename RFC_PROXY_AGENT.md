# Public Cluster verification problems

## Original Problems/Purpose

In the original design of the Spore Protocol, if one wants to create a spore with a Cluster ID, then it must follow:

1. The referenced Cluster Cell must be found in the `CellDeps`.
2. The referenced Cluster Cell must be found in the `Inputs`.
3. The referenced Cluster Cell must be found in the `Outputs`.

This is for verifying the ownership of the referenced Cluster Cell, avoiding vicious behavious happened to one’s private Cluster.

While this guarantees unexpected minting will not happen to a certain Cluster, it also brings a problem for Clusters that support public minting — those using specific lock to achieve this. That is, there might be a  bottleneck when the Cluster contains many popular Spore cells and there's only one Cluster cell available to construct transactions.

More specificly, if two person wants to mint different Spores into a Cluster named `Cluster A`, and they accidently send their mint transaction at the same time, then one of their transaction will be rejected because of the Cluster was consumed and recreated, the rejected one also needs to reconstruct its transaction in order to continue minting.

## Solution details

### Step1: Creating Cluster Proxy Cell

In this stage, the **owner** needs to create a special cell, here we called it **Cluster Proxy Cell**

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

Below is a transaction shows how to create a Cluster Proxy Cell, by putting a Cluster Cell to Inputs & Outputs:

```yaml
# Method 1, direct input
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

Or you can use an **Input Cell** with same Lock to the Cluster Cell to achieve this

```yaml
# Method 2, use lock proxy
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

Below shows the methods about how to create a Cluster Proxy Agent Cell:

```yaml
# Method 1, direct input
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

Or you can make a payment to the Cluster Proxy owner to achieve the same:  (By transfering capacity to the same lock address with Cluster Proxy)

```yaml
# Method 2, with payment appearance
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

In here, payment cell is just an example showcase, it can be any unlockable cell and is not limited in just one cell.  

### Step3: Mint Spore with Cluster Agent

Holder of the Cluster Agent Cell can now mint Spore using this Cell. Valid methods are listed below.

1. **Mint with direct input**

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

2. **Mint with Lock Proxy**

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

3. **Mint with Signature** (not implemented)

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
