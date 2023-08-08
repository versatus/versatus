use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::Parser;
use telemetry::info;
use wasm_runtime::wasm_runtime::WasmRuntime;

#[derive(Parser, Debug)]
pub struct ExecuteOpts {
    /// The path to the WASM object file to load and describe
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub wasm: PathBuf,
    /// The path to a JSON file to become input to the running WASM module
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub json: PathBuf,
    /// An environment variable to pass to the running WASM module. May be used
    /// multiple times.
    #[clap(short, long, value_parser, value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    /// Remaining arguments (after '--') are passed to the WASM module command
    /// line.
    #[clap(last = true)]
    pub args: Vec<String>,
}

/// Read and parse a WASM object and print high level information that is
/// targeted toward developers of WASM modules. It should attempt to describe
/// how the module might, or might not, be viable as an off-chain smart contract
/// compute job.
pub fn run(opts: &ExecuteOpts) -> Result<()> {
    let wasmfile = opts
        .wasm
        .to_str()
        .ok_or(anyhow!("Failed to convert WASM filename to valid string."))?;
    let jsonfile = opts
        .json
        .to_str()
        .ok_or(anyhow!("Failed to convert JSON filename to valid string."))?;
    let wasm_bytes = std::fs::read(wasmfile)?;
    info!(
        "Loaded {} bytes of WASM data from {} to execute.",
        wasm_bytes.len(),
        wasmfile
    );
    let json_data = std::fs::read(jsonfile)?;
    info!(
        "Loaded {} bytes of JSON data from {} as input.",
        json_data.len(),
        jsonfile
    );

    let mut env_vars: HashMap<String, String> = HashMap::new();
    for var in opts.env.iter() {
        if let Some((key, value)) = var.split_once('=') {
            env_vars.insert(key.to_string(), value.to_string());
        }
    }

    // Execute the WASM module.
    let mut wasm = WasmRuntime::new(&wasm_bytes)?
        .stdin(&json_data)?
        .env(&env_vars)?
        .args(&opts.args)?;
    wasm.execute()?;

    // Temporary output for user -- will eventually be more structured and both
    // human and machine readable.
    println!("{}", &wasm.stdout());
    eprintln!("Contract errors: {}", &wasm.stderr());

    Ok(())
}
