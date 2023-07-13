use std::path::PathBuf;

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
pub fn run(opts: &DescribeOpts) -> anyhow::Result<()> {
    let filename = opts.wasm.to_str().expect("Need path name");
    println!("Running describe for {}", filename);
    let w = WasmLoaderBuilder::default()
        .wasm_bytes(std::fs::read(filename).unwrap())
        .parse()
        .unwrap()
        .build()?;

    println!("WASM?  {}", w.from_wat);
    println!("WASI?  {}", w.is_wasi);
    println!("WASIX? {}", w.is_wasix);
    println!("Javy?  {}", w.needs_javy);
    println!("Start? {}", w.has_start);
    println!("VRRB?  {}", w.has_vrrb);
    println!("Namespaces: {:?}", w.imports.keys());

    Ok(())
}
