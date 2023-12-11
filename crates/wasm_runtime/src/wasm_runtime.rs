//! Web Assembly runtime execution
//!
//! This is the WASM runtime for the VRRB compute stack. It allows WASI/WASIX
//! function calls and assumes that the WASM payload has a _start entry point,
//! reads from STDIN and writes to STDOUT. It wraps around the Wasmer WASM
//! runtime.

use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::Arc,
};

use super::{
    limiting_tunables::{LimitingTunables, DEFAULT_PAGE_LIMIT},
    metering::MeteringConfig,
};
use telemetry::debug;
use wasmer::{
    wasmparser::Operator, BaseTunables, CompilerConfig, Engine, Instance, Module, NativeEngineExt,
    Store, Target,
};
use wasmer_middlewares::metering::get_remaining_points;
use wasmer_wasix::{Pipe, WasiEnv};

/// This is the first command line argument, traditionally reserved for the
/// program name (argv[0] in C and others).
const MODULE_ARGV0: &str = "vrrb-contract";

use crate::errors::WasmRuntimeError;
pub type RuntimeResult<T> = Result<T, WasmRuntimeError>;

pub struct WasmRuntime {
    store: Store,
    module: Module,
    stdin: Vec<u8>,
    stdout: String,
    stderr: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}
#[allow(clippy::result_large_err)]
impl WasmRuntime {
    /// Creates a new WasmRuntime environment to execute the WASM binary passed
    /// in.
    ///
    /// C represents the compiler to use, which at the time of writing is Cranelift.
    pub fn new<C>(
        target: &Target,
        wasm_bytes: &[u8],
        metering_config: MeteringConfig<impl Fn(&Operator<'_>) -> u64 + Send + Sync + 'static>,
    ) -> RuntimeResult<Self>
    where
        C: Default + Into<Engine> + CompilerConfig,
    {
        // Setup Tunables
        let mut compiler = C::default();
        compiler.push_middleware(Arc::new(metering_config.into_metering()));
        let base = BaseTunables::for_target(target);
        let tunables = LimitingTunables::new(base, DEFAULT_PAGE_LIMIT);
        let mut engine: Engine = compiler.into();
        engine.set_tunables(tunables);
        // Create an in-memory store for everything required to compile and run a WASM
        // module
        let store = Store::new(engine);

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
    pub fn args(mut self, args: &[String]) -> Self {
        self.args = args.to_vec();
        self
    }

    /// Optionally sets environment variables for the running WASM module.
    pub fn env(mut self, env_vars: &HashMap<String, String>) -> Self {
        self.env = env_vars.clone();
        self
    }

    /// Writes a vector of bytes to stdin for the WASM module on execution.
    pub fn stdin(mut self, input: &[u8]) -> Self {
        self.stdin = input.to_vec();
        self
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
    pub fn execute(&mut self) -> RuntimeResult<()> {
        let (mut stdin, in_wasm) = Pipe::channel();
        let (out_wasm, mut stdout) = Pipe::channel();
        let (err_wasm, mut stderr) = Pipe::channel();
        stdin.write_all(&self.stdin)?;
        stdin.flush()?;

        self.init_wasi_fn_env((in_wasm, out_wasm, err_wasm))?;

        stdout.read_to_string(&mut self.stdout)?;
        stderr.read_to_string(&mut self.stderr)?;
        Ok(())
    }

    fn init_wasi_fn_env(
        &mut self,
        (in_wasm, out_wasm, err_wasm): (Pipe, Pipe, Pipe),
    ) -> RuntimeResult<()> {
        let store = &mut self.store;
        let module = &self.module;
        let mut wasi_fn_env = WasiEnv::builder(MODULE_ARGV0)
            .stdin(Box::new(in_wasm))
            .stdout(Box::new(out_wasm))
            .stderr(Box::new(err_wasm))
            .args(Box::new(self.args.iter()))
            .envs(Box::new(self.env.iter()))
            .finalize(store)?;

        let import_obj = wasi_fn_env.import_object(store, module)?;
        let instance = Instance::new(store, module, &import_obj)?;

        let mem_view = instance.exports.get_memory("memory")?.view(store);
        telemetry::info!("Memory: {:?}", mem_view.size());

        wasi_fn_env.initialize(store, instance.clone())?;
        let start = instance.exports.get_function("_start")?;
        start.call(store, &[])?;

        telemetry::info!(
            "MeteringPoints::{:?}",
            get_remaining_points(store, &instance)
        );

        wasi_fn_env.cleanup(store, None);
        Ok(())
    }
}
