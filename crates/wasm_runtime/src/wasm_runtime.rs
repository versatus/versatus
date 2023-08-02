//! Web Assembly runtime execution
//!
//! This is the WASM runtime for the VRRB compute stack. It allows WASI/WASIX
//! function calls and assumes that the WASM payload has a _start entry point,
//! reads from STDIN and writes to STDOUT. It wraps around the Wasmer WASM
//! runtime.

use std::{
    collections::HashMap,
    io::{Read, Write},
};

use anyhow::Result;
use telemetry::debug;
use wasmer::{Module, Store};
use wasmer_wasix::{Pipe, WasiEnv};

/// This is the first command line argument, traditionally reserved for the
/// program name (argv[0] in C and others).
const MODULE_ARGV0: &str = "vrrb-contract";

pub struct WasmRuntime {
    store: Store,
    module: Module,
    stdin: Vec<u8>,
    stdout: String,
    stderr: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

impl WasmRuntime {
    /// Creates a new WasmRuntime environment to execute the WASM binary passed
    /// in.
    pub fn new(wasm_bytes: &Vec<u8>) -> Result<Self> {
        // Create an in-memory store for everything required to compile and run a WASM
        // module
        let store = Store::default();

        debug!("Compiling {} bytes of WASM", wasm_bytes.len());

        // Compile module into in-memory store
        let module = Module::new(&store, wasm_bytes)?;
        Ok(Self {
            store,
            module,
            stdin: vec![],
            stdout: String::new(),
            stderr: String::new(),
            args: vec![],
            env: HashMap::new(),
        })
    }

    /// Adds a set of command line arguments to the WASM module's execution
    pub fn args(mut self, args: &[String]) -> Result<Self> {
        self.args = args.to_vec();
        Ok(self)
    }

    /// Optionally sets environment variables for the running WASM module.
    pub fn env(mut self, env_vars: &HashMap<String, String>) -> Result<Self> {
        self.env = env_vars.clone();
        Ok(self)
    }

    /// Writes a vector of bytes to stdin for the WASM module on execution.
    pub fn stdin(mut self, input: &[u8]) -> Result<Self> {
        self.stdin = input.to_vec();
        Ok(self)
    }

    /// Returns a string containing the output written to the WASM module's
    /// stdout stream.
    pub fn stdout(&self) -> String {
        self.stdout.clone()
    }

    /// Returns a string containing the output written to the WASM module's
    /// stderr stream.
    pub fn stderr(&self) -> String {
        self.stderr.clone()
    }

    /// Execute the compiled WASM module and retrieve the result.
    pub fn execute(&mut self) -> Result<()> {
        let (mut stdin, in_wasm) = Pipe::channel();
        let (out_wasm, mut stdout) = Pipe::channel();
        let (err_wasm, mut stderr) = Pipe::channel();
        stdin.write_all(&self.stdin)?;
        stdin.flush()?;
        WasiEnv::builder(MODULE_ARGV0)
            .stdin(Box::new(in_wasm))
            .stdout(Box::new(out_wasm))
            .stderr(Box::new(err_wasm))
            .args(Box::new(self.args.iter()))
            .envs(Box::new(self.env.iter()))
            .run_with_store(self.module.to_owned(), &mut self.store)?;
        stdout.read_to_string(&mut self.stdout)?;
        stderr.read_to_string(&mut self.stderr)?;
        Ok(())
    }
}
