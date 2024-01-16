use anyhow::{anyhow, Result};
use bonsaidb::{
    core::{
        connection::StorageConnection,
        schema::{SerializedCollection, SerializedView},
    },
    local::Storage,
};
use clap::Parser;
use serde_json;
use std::{collections::HashMap, path::PathBuf, time::SystemTime};
use telemetry::info;
use versatus_rust::{
    eip20::Erc20Result,
    versatus_rust::{
        AccountInfo, Address, ContractInputs, ContractResult, ProtocolInputs, SmartContractInputs,
        SmartContractOutputs,
    },
};
use wasm_runtime::{
    metering::{cost_function, MeteringConfig},
    wasm_runtime::WasmRuntime,
};
use wasmer::{Cranelift, Target};

use crate::commands::testinitdb;

use super::{
    testbalance::get_balance,
    testinitdb::{AccountAddress, AccountBalance},
};

#[derive(Parser, Debug)]
pub struct TestContractOpts {
    /// This is the path to the database to be created/used. #716, this path is what we'll feed
    /// into the database driver.
    #[clap(short, long)]
    pub dbpath: String,
    /// The path to the WASM object file to load and describe
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub wasm: PathBuf,
    /// The function to call within the smart contract. #716 this will influence the JSON we
    /// generate below to pass into the smart contract when we execute it. TODO: mg@ needs to also
    /// remember to add some function-specific arguments here to allow those to be passed in.
    #[clap(short, long, value_parser, value_name = "FUNCTION")]
    pub function: String,
    /// The arguments to pass into the function as a JSON object. See the `versatus-rust` github
    /// repository for the inputs that supported functions take. For now, this is a string
    /// interpretted as a JSON object, whereas later, it'll likely be more formal. #716, this JSON
    /// will equate to the data in the FunctionInputs enum here:
    /// https://github.com/versatus/versatus-rust/blob/main/src/versatus_rust.rs#L94
    #[clap(short, long, value_parser, value_name = "JSON", default_value = "[]")]
    pub inputs: String,
    /// An environment variable to pass to the running WASM module. May be used
    /// multiple times.
    #[clap(short, long, value_parser, value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    /// The initial limit of credits that the WASM module's meter will use to track
    /// operation expenses.
    #[clap(short = 'l', long, value_parser, value_name = "UINT64")]
    pub meter_limit: u64,
    /// Remaining arguments (after '--') are passed to the WASM module command
    /// line.
    #[clap(last = true)]
    pub args: Vec<String>,
}

/// Read and parse a WASM object and print high level information that is
/// targeted toward developers of WASM modules. It should attempt to describe
/// how the module might, or might not, be viable as an off-chain smart contract
/// compute job.
pub fn run(opts: &TestContractOpts) -> Result<()> {
    let wasmfile = opts
        .wasm
        .to_str()
        .ok_or(anyhow!("Failed to convert WASM filename to valid string."))?;
    let wasm_bytes = std::fs::read(wasmfile)?;
    info!(
        "Loaded {} bytes of WASM data from {} to execute.",
        wasm_bytes.len(),
        wasmfile
    );
    let mut env_vars: HashMap<String, String> = HashMap::new();
    for var in opts.env.iter() {
        if let Some((key, value)) = var.split_once('=') {
            env_vars.insert(key.to_string(), value.to_string());
        }
    }

    let storage_connection = testinitdb::open_storage(&opts.dbpath)?;

    let target = Target::default();
    // Test the WASM module.
    let mut wasm = WasmRuntime::new::<Cranelift>(
        &target,
        &wasm_bytes,
        MeteringConfig::new(opts.meter_limit, cost_function),
    )?
    .stdin(
        &serde_json::to_string(&create_contract_inputs(
            &opts.function,
            &opts.inputs,
            &storage_connection,
        )?)?
        .into_bytes(),
    )
    .env(&env_vars)
    .args(&opts.args);
    wasm.execute()?;
    let contract_outputs: &SmartContractOutputs = &serde_json::from_str(&wasm.stdout())?;
    let updated_contract_outputs = update_db(&storage_connection, contract_outputs);

    // #716 We shouldn't print the output here, but rather parse it and use it to update the
    // database. For example, if an ErcTransferEvent is part of the output(https://github.com/versatus/versatus-rust/blob/main/src/eip20.rs#L48), we should move the balance from the from account to the to account.
    println!("{:?}", &updated_contract_outputs);
    // TODO: Update storage with the output of the contract
    // using the overwrite method in SerializedCollection
    // similarly to the use of insert_into for AccountBalance
    // in the testinitdb module.
    eprintln!("Contract errors: {}", &wasm.stderr());

    Ok(())
}

fn update_db(storage_connection: &Storage, contract_outputs: &SmartContractOutputs) -> Result<()> {
    for res in contract_outputs.result.iter() {
        if let ContractResult::Erc20(erc20_res) = &res {
            match &erc20_res {
                // Update entire DB on Transfer call.
                // Uses ContractResults to update AccountInfo & updates ProtocolInputs via DB insertion method.
                Erc20Result::Transfer(transfer) => {
                    let accounts_db =
                        storage_connection.database::<AccountBalance>("account-balance")?;
                    let transfer_amount = transfer.value;
                    let from_account = AccountAddress {
                        address: transfer.from.0.clone(),
                    };
                    let to_account = AccountAddress {
                        address: transfer.to.0.clone(),
                    };
                    let from_balance = get_balance(&from_account, storage_connection)?;
                    let to_balance = get_balance(&to_account, storage_connection)?;
                    if from_balance.contents.value >= transfer_amount {
                        AccountBalance {
                            value: from_balance.contents.value - transfer_amount,
                        }
                        .overwrite_into(&from_account, &accounts_db)?;
                        AccountBalance {
                            value: to_balance.contents.value + transfer_amount,
                        }
                        .overwrite_into(&to_account, &accounts_db)?;

                        let protocol_db = storage_connection
                            .database::<testinitdb::ProtocolInputs>("protocol-inputs")?;
                        let (version, (block_height, _)) =
                            get_protocol_inputs(&storage_connection)?;
                        let system_time = std::time::SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map_err(|e| anyhow!("{e:?}"))?;
                        testinitdb::ProtocolInputs::insert(
                            &protocol_db,
                            version,
                            block_height + 1,
                            system_time.as_secs(),
                        )?;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
// #716 This JSON string is a placeholder to allow the code to compile. What we need to do is
// to build the JSON to match the JSON generated by the example in the versatus-rust
// repository, but build it from the contents of the database and command line inputs. We can
// assume (for now) that all contracts will be ERC20 when dealing with inputs and outputs.
fn create_contract_inputs(
    function: &str,
    inputs: &str,
    storage_connection: &bonsaidb::local::Storage,
) -> Result<SmartContractInputs> {
    let (version, (block_height, block_time)) = get_protocol_inputs(storage_connection)?;
    let raw_address = Address([2; 20]); // TODO: get this from the command line args?
    let account_address = AccountAddress {
        address: raw_address.0,
    };
    let balance_document = get_balance(&account_address, storage_connection)?;
    let account_balance = balance_document.contents.value;

    Ok(SmartContractInputs {
        version,
        account_info: AccountInfo {
            account_address: raw_address, // TODO: is this the sender or receiver?
            account_balance,
        },
        protocol_input: ProtocolInputs {
            version,
            block_height,
            block_time,
        },
        contract_input: ContractInputs {
            contract_fn: function.to_owned(),
            function_inputs: serde_json::from_str(inputs) // deserialize json into FunctionInputs
                .map_err(|e| anyhow!("failed to deserialize function inputs: {e:?}"))?,
        },
    })
}

fn get_protocol_inputs(storage_connection: &Storage) -> Result<(i32, (u64, u64))> {
    let protocol_db =
        storage_connection.database::<testinitdb::ProtocolInputs>("protocol-inputs")?;
    let protocol_view = testinitdb::ProtocolView::entries(&protocol_db)
        .ascending()
        .query()?;
    let protocol_document = protocol_view
        .last()
        .expect("found empty protocol inputs database, initialize the test db and try again");
    let latest_version = protocol_document.key;
    let (block_height, block_time) = protocol_document.value;
    Ok((latest_version, (block_height, block_time)))
}

#[cfg(test)]
mod contract_tests {
    use crate::commands::{testcontract, testinitdb};
    use std::path::PathBuf;

    #[test]
    fn test_create_contract_inputs() {
        let storage =
            testinitdb::open_storage(&"./bonsaidb".to_string()).expect("could not open storage");
        let contract_inputs = testcontract::create_contract_inputs(
            "transfer",
            "{\
                \"erc20\": {
                    \"transfer\": {
                    \"value\": \"0xffff\",
                    \"address\": \"0x0303030303030303030303030303030303030303\"
                    }
                }
            }",
            &storage,
        );
        assert!(contract_inputs.is_ok());
    }

    #[test]
    fn test_contract() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("src/commands/testcontract/test_data/erc20.wasm");

        let res = testcontract::run(&testcontract::TestContractOpts {
            dbpath: testinitdb::DEFAULT_DB_PATH.to_string(),
            wasm: d,
            function: "transfer".to_string(),
            inputs: "{\
                \"erc20\": {
                    \"transfer\": {
                    \"value\": \"0xffff\",
                    \"address\": \"0x0303030303030303030303030303030303030303\"
                    }
                }
            }"
            .to_string(),
            env: vec![],
            meter_limit: 1000,
            args: vec![],
        });
        dbg!(&res);
        assert!(res.is_ok());
    }
}
