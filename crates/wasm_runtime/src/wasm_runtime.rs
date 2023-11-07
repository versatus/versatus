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
use wasmer::{AsStoreMut, Module, Store};
use wasmer_wasix::{Pipe, WasiEnv, WasiEnvBuilder, WasiError, WasiRuntimeError};
use wasmer_wasix_types::wasi::{Errno, ExitCode};

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
            .run_with_store_and_report(self.module.to_owned(), &mut self.store)?;
        stdout.read_to_string(&mut self.stdout)?;
        stderr.read_to_string(&mut self.stderr)?;
        Ok(())
    }
}

pub(crate) trait MemChecker {
    /// Implements the same logic as `WasiEnv::run_with_store` but
    /// queries memory from the internal instance created within.
    #[allow(clippy::result_large_err)]
    fn run_with_store_and_report(
        self,
        module: Module,
        store: &mut Store,
    ) -> Result<(), WasiRuntimeError>;
}

impl MemChecker for WasiEnvBuilder {
    fn run_with_store_and_report(
        mut self,
        module: Module,
        store: &mut Store,
    ) -> Result<(), WasiRuntimeError> {
        if self
            .capabilities_mut()
            .threading
            .enable_asynchronous_threading
        {
            telemetry::warn!(
                "The enable_asynchronous_threading capability is enabled. Use WasiEnvBuilder::run_with_store_async() to avoid spurious errors.",
            );
        }

        let (instance, env) = self.instantiate(module, store)?;

        let start = instance.exports.get_function("_start")?;
        let mem_view = instance.exports.get_memory("memory")?.view(&store);
        telemetry::info!(
            "Memory view:
    pages: {:?}
    bytes: {:?}
    data_size: {}",
            mem_view.size(),
            mem_view.size().bytes(),
            mem_view.data_size(),
        );
        env.data(&store).thread.set_status_running();

        let result = run_wasi_func_start(start, store);
        let (result, exit_code) = wasi_exit_code(result);

        let pid = env.data(&store).pid();
        let tid = env.data(&store).tid();
        telemetry::trace!(
            %pid,
            %tid,
            %exit_code,
            error=result.as_ref().err().map(|e| e as &dyn std::error::Error),
            "main exit",
        );

        env.cleanup(store, Some(exit_code));

        result
    }
}

/// Run a main function.
///
/// This is usually called "_start" in WASI modules.
/// The function will not receive arguments or return values.
///
/// An exit code that is not 0 will be returned as a `WasiError::Exit`.
#[allow(clippy::result_large_err)]
pub(crate) fn run_wasi_func_start(
    func: &wasmer::Function,
    store: &mut impl AsStoreMut,
) -> Result<(), WasiRuntimeError> {
    run_wasi_func(func, store, &[])?;
    Ok(())
}

#[allow(clippy::result_large_err)]
pub(crate) fn run_wasi_func(
    func: &wasmer::Function,
    store: &mut impl AsStoreMut,
    params: &[wasmer::Value],
) -> Result<Box<[wasmer::Value]>, WasiRuntimeError> {
    func.call(store, params).map_err(|err| {
        if let Some(_werr) = err.downcast_ref::<WasiError>() {
            let werr = err.downcast::<WasiError>().unwrap();
            WasiRuntimeError::Wasi(werr)
        } else {
            WasiRuntimeError::Runtime(err)
        }
    })
}

/// Extract the exit code from a `Result<(), WasiRuntimeError>`.
///
/// We need this because calling `exit(0)` inside a WASI program technically
/// triggers [`WasiError`] with an exit code of `0`, but the end user won't want
/// that treated as an error.
fn wasi_exit_code(
    mut result: Result<(), WasiRuntimeError>,
) -> (Result<(), WasiRuntimeError>, ExitCode) {
    let exit_code = match &result {
        Ok(_) => Errno::Success.into(),
        Err(err) => match err.as_exit_code() {
            Some(code) if code.is_success() => {
                // This is actually not an error, so we need to fix up the
                // result
                result = Ok(());
                Errno::Success.into()
            },
            Some(other) => other,
            None => Errno::Noexec.into(),
        },
    };

    (result, exit_code)
}
