use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use wasm_loader::wasm_loader::WasmLoaderBuilder;
use wasm_runtime::limiting_tunables::DEFAULT_PAGE_LIMIT;

#[derive(Parser, Debug)]
pub struct ValidateOpts {
    /// The path to the WASM object file to load and validate
    #[clap(short, long, value_parser, value_name = "FILE")]
    wasm: PathBuf,
}

/// Constants for a currently support namespaces.
// XXX: If we keep this, move the constants and the check to the wasm_loader
// crate as an associated function rather than keep the logic here.
const ENV: &str = "env";
const WASI_SNAPSHOT_PREVIEW1: &str = "wasi_snapshot_preview1";
const SUPPORTED_NAMESPACES: &[&str] = &[ENV, WASI_SNAPSHOT_PREVIEW1];

pub fn run(opts: &ValidateOpts) -> Result<()> {
    let mut expected_to_run = true;
    let filename = opts.wasm.to_str().expect("Need path name");
    println!("Running validation for {}", filename);
    let w = WasmLoaderBuilder::default()
        .wasm_bytes(
            std::fs::read(filename)
                .map_err(|e| anyhow::Error::msg(format!("Error reading Wasm file: {}", e)))?,
        )
        .parse()
        .map_err(|e| anyhow::Error::msg(format!("Error parsing Wasm file: {}", e)))?
        .build()?;

    if w.wasm_version != 1 {
        println!("WASM Version {} not yet supported.", w.wasm_version);
        expected_to_run = false;
    }

    let limit: u64 = DEFAULT_PAGE_LIMIT.0.into();
    if w.wasm_memory > limit {
        println!(
            "WASM module needs {} pages of memory. Current limit for contracts is {}.",
            w.wasm_memory, limit
        );
        expected_to_run = false;
    }

    if !w.is_wasi && !w.is_wasix {
        println!("WASM module isn't built for use with WASI/WASIX");
        expected_to_run = false;
    }

    if !w.has_start {
        println!("WASM module doesn't have an entry point exported as _start");
        expected_to_run = false;
    }

    if !w.has_versatus {
        // This, unlike the other checks, is not fatal
        println!("WASM module doesn't make use of any Versatus extensions (not fatal)");
    }

    if w.needs_javy {
        println!("WASM module is dynamically linked against Javy runtime. Should be static");
        expected_to_run = false;
    }

    let mut extra_namespaces = vec![];
    for key in w.imports.keys() {
        if !SUPPORTED_NAMESPACES.contains(&key.as_str()) {
            extra_namespaces.push(key);
        }
    }
    if !extra_namespaces.is_empty() {
        println!(
            "Found references to unexpected namespaces: {:?}",
            extra_namespaces
        );
        expected_to_run = false;
    }

    if expected_to_run {
        println!("WASM module is expected to run under the Versatus runtime");
    } else {
        println!("WASM module is not expected to run under the Versatus runtime");
    }

    Ok(())
}
