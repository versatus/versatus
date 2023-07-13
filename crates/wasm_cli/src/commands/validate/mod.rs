use std::path::PathBuf;

use clap::Parser;
use wasm_loader::wasm_loader::WasmLoaderBuilder;

#[derive(Parser, Debug)]
pub struct ValidateOpts {
    /// The path to the WASM object file to load and validate
    #[clap(short, long, value_parser, value_name = "FILE")]
    wasm: PathBuf,
}

pub fn run(opts: &ValidateOpts) -> anyhow::Result<()> {
    let supported_namespaces: Vec<String> =
        vec!["env".to_string(), "wasi_snapshot_preview1".to_string()];
    let mut expected_to_run = true;
    let filename = opts.wasm.to_str().expect("Need path name");
    println!("Running describe for {}", filename);
    let w = WasmLoaderBuilder::default()
        .wasm_bytes(std::fs::read(filename).unwrap())
        .parse()
        .unwrap()
        .build()?;

    if !w.is_wasi && !w.is_wasix {
        println!("WASM module isn't built for use with WASI/WASIX");
        expected_to_run = false;
    }

    if !w.has_start {
        println!("WASM module doesn't have an entry point exported as _start");
        expected_to_run = false;
    }

    if !w.has_vrrb {
        // This, unlike the other checks, is not fatal
        println!("WASM module doesn't make use of any VRRB extensions (not fatal)");
    }

    let mut extra_namespaces = vec![];
    for key in w.imports.keys().into_iter() {
        if !supported_namespaces.contains(key) {
            extra_namespaces.push(key);
        }
    }
    if extra_namespaces.len() != 0 {
        println!(
            "Found references to unexpected namespaces: {:?}",
            extra_namespaces
        );
        expected_to_run = false;
    }

    if expected_to_run {
        println!("WASM module is expected to run under the VRRB runtime");
    } else {
        println!("WASM module is not expected to run under the VRRB runtime");
    }

    Ok(())
}
