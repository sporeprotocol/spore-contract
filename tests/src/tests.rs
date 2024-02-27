mod cluster;
mod mutant;
mod spore;
mod utils;

#[cfg(test)]
mod xxx {
    use std::str::FromStr;

    use ckb_sdk::{Address, CkbRpcClient};
    use ckb_testtool::ckb_jsonrpc_types::{Either, TransactionView as JsonTxView};
    use ckb_testtool::ckb_types::core::{DepType, TransactionView};
    use ckb_testtool::ckb_types::h256;
    use ckb_testtool::ckb_types::packed::{CellDep, CellInput, CellOutput, OutPoint};
    use ckb_testtool::ckb_types::prelude::{Builder, Entity, Pack};

    #[test]
    fn burn_spore_contract() {
        let tx_hash = h256!("0xcb67e11a39594a183eae50664073a85fa0140ae25480df61114aca0e47726d38");
        let index = 0u32;
        let payee_address = "ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqfqmf4hphl9jkrw3934mwe6m3a2nx88rzgrd9gqh";
        let network = "https://mainnet.ckb.dev/";

        // fetch deployed spore contract cell
        let rpc = CkbRpcClient::new(network);
        let raw_tx = rpc
            .get_transaction(tx_hash.clone())
            .unwrap()
            .unwrap()
            .transaction
            .unwrap()
            .inner;
        let tx = match raw_tx {
            Either::Left(tx) => tx,
            Either::Right(bytes) => serde_json::from_slice(bytes.as_bytes()).unwrap(),
        };
        let spore_contract_cell = tx.inner.outputs.get(index as usize).unwrap();
        println!("spore contract cell: {spore_contract_cell:?}");

        // confirm payee address
        let address = Address::from_str(payee_address).unwrap();
        println!("address payload: {address:?}");

        // assemble burn transaction
        let input_cell = CellInput::new_builder()
            .previous_output(
                OutPoint::new_builder()
                    .tx_hash(tx_hash.pack())
                    .index(index.pack())
                    .build(),
            )
            .build();
        let capacity: u64 = spore_contract_cell.capacity.into();
        let output_cell = CellOutput::new_builder()
            .lock(address.payload().into())
            .capacity((capacity - 100_000_000u64).pack())
            .build();
        let dep_cell = CellDep::new_builder()
            .out_point(
                OutPoint::new_builder()
                    .tx_hash(
                        h256!("0x71a7ba8fc96349fea0ed3a5c47992e3b4084b031a42264a018e0072e8172e46c")
                            .pack(),
                    )
                    .index(0u32.pack())
                    .build(),
            )
            .dep_type(DepType::DepGroup.into())
            .build();
        let tx = TransactionView::new_advanced_builder()
            .input(input_cell)
            .output(output_cell)
            .output_data(Default::default())
            .cell_dep(dep_cell)
            .build();
        let json_tx = serde_json::to_string_pretty(&JsonTxView::from(tx)).unwrap();
        println!("tx = {json_tx}");
        std::fs::write("../deployment/tx.json", json_tx).unwrap();
    }
}
