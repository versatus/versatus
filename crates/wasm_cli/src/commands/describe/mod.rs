use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;
use wasm_loader::wasm_loader::WasmLoaderBuilder;

#[derive(Parser, Debug)]
pub struct DescribeOpts {
    /// The path to the WASM object file to load and describe
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub wasm: PathBuf,
}

/// Read and parse a WASM object and print high level information that is
/// targetted toward developers of WASM modules. It should attempt to describe
/// how the module might, or might not, be viable as an off-chain smart contract
/// compute job.
pub fn run(opts: &DescribeOpts) -> Result<()> {
    let filename = opts
        .wasm
        .to_str()
        .ok_or(anyhow!("Failed to convert filename to valid string."))?;
    println!("Running describe for {}", filename);
    let wasm_loader = WasmLoaderBuilder::from_filename(filename)?;

    println!("WASM?  {}", wasm_loader.from_wat);
    println!("WASI?  {}", wasm_loader.is_wasi);
    println!("WASIX? {}", wasm_loader.is_wasix);
    println!("Javy?  {}", wasm_loader.needs_javy);
    println!("Start? {}", wasm_loader.has_start);
    println!("VRRB?  {}", wasm_loader.has_vrrb);
    println!("Namespaces: {:?}", wasm_loader.imports.keys());

    Ok(())
}
