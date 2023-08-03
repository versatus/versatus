//! Web Assembly loader and validator
//!
//! Provides some basic functionality to load Web Assembly bytes (or WAT
//! strings) from files or other locations and perform some basic VRRB sanity
//! checking or inspection of the loaded module(s).

use std::collections::HashMap;

use anyhow::Result;
use derive_builder::Builder;
// Use and review of log macros within this crate:
//   * error for *user-actionable* information to be visible to a developer or operator
//   * info for information that is useful to a compute developer or operator, but not fatal
//   * debug for information useful to maintainers in being able to remotely troubleshoot
//     issues
use telemetry::log::{debug, error};
use wasmer::wat2wasm;
use wasmparser::{Parser, Payload};

use crate::constants;

/// A struct to represent some loaded and parsed WASM.
#[derive(Default, Debug, Clone, Builder)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct WasmLoader {
    /// The bytes of WASM themselves, potentially having been translated
    /// from WAT.
    pub wasm_bytes: Vec<u8>,
    /// Represents the size of the WASM binary.
    #[builder(default = "0")]
    #[builder(private)]
    pub wasm_size: usize,
    // XXX: look at a cleaner interface for all of these booleans as we get a better handle on
    // what that interface should be able to expose or how it's most likely to end up being used.
    /// True if this module uses WASI interfaces. True for
    /// [WASI_NAMESPACE_UNSTABLE], [WASI_NAMESPACE_PREVIEW1],
    /// [WASIX_NAMESPACE_32V1] and [WASIX_NAMESPACE_64V1].
    #[builder(default = "false")]
    #[builder(private)]
    pub is_wasi: bool,
    /// True if this WASM module uses the WASIX interfaces. True for
    /// [WASIX_NAMESPACE_32V1] and [WASIX_NAMESPACE_64V1].
    #[builder(default = "false")]
    #[builder(private)]
    pub is_wasix: bool,
    /// True if this WASM module was compiled with Javy and needs the
    /// quickjs library.
    #[builder(default = "false")]
    #[builder(private)]
    pub needs_javy: bool,
    /// A string of Web Assembly Text (WAT) to be compiled to WASM.
    #[builder(default = "vec![]")]
    pub wat_text: Vec<u8>,
    /// True if this WASM was loaded as WAT and translated to WASM.
    #[builder(default = "false")]
    #[builder(private)]
    pub from_wat: bool,
    /// True if this WASM module exports a _start symbol
    #[builder(default = "false")]
    #[builder(private)]
    pub has_start: bool,
    /// True if this WASM module exports any of the VRRB magic symbols
    #[builder(default = "false")]
    #[builder(private)]
    pub has_vrrb: bool,
    /// A HashMap where the key is a string representing the namespace, and
    /// the value is a set of strings representing symbols this
    /// module expects to be present. Used in determining
    /// dependencies, etc.
    #[builder(private)]
    pub imports: HashMap<String, Vec<String>>,
}

impl WasmLoaderBuilder {
    /// Performs some validation on the built WasmLoader struct. Called
    /// automatically as part of [WasmLoaderBuilder::build].
    fn validate(&self) -> Result<(), String> {
        // validate that we have some bytes that look WASMey.
        match &self.wasm_bytes {
            Some(b) => {
                // Try to match the magic header bytes
                if self.contains_magic(b) {
                    debug!("WASM header looks legit");
                    Ok(())
                } else {
                    error!("Invalid WASM bytes provided");
                    Err("WASM Magic header invalid".to_string())
                }
            },
            None => {
                error!("No WASM bytes found");
                Err("WASM Magic header not found".to_string())
            },
        }

        // Is unreachable
    }

    /// Build a WasmLoader given the path to a filename.
    pub fn from_filename(filename: &str) -> Result<WasmLoader> {
        Ok(WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(filename)?)
            .parse()?
            .build()?)
    }

    /// Simple function to compare the first four bytes of an array with the
    /// well-known WASM magic string, '\0asm'.
    fn contains_magic(&self, bytes: &[u8]) -> bool {
        let header = &bytes[0..4];

        debug!("WASM header missing: {:02x?}", header);
        // return whether the header is equal to the [WASM_MAGIC] constant.
        header == constants::WASM_MAGIC
    }

    /// Parses the provided WASM binary and collects hints about requirements,
    /// dependencies, interface versions, etc. Must be called to create a valid
    /// object. Should be called right before build() and after wasm_bytes().
    pub fn parse(&mut self) -> Result<Self> {
        let mut new = self.clone();
        let mut imports: HashMap<String, Vec<String>> = HashMap::new();

        // If we have WAT text, attempt to compile it into wasm_bytes[]
        debug!("Checking for WAT");
        if let Some(wat) = &self.wat_text {
            debug!("Found WASM Text -- attempting to compile to WASM");
            new.wasm_bytes = Some(wat2wasm(wat)?.into_owned());
            new.from_wat = Some(true);
        }

        if let Some(wasm) = &self.wasm_bytes {
            for payload in Parser::new(constants::WASM_PARSE_OFFSET).parse_all(wasm) {
                match payload {
                    Ok(p) => {
                        match p {
                            Payload::Version { .. } => {
                                debug!("WASM Version Section");
                            },
                            Payload::ExportSection(s) => {
                                for export in s {
                                    let export = export?;

                                    debug!("Export: {:?}", export);

                                    if export.name == constants::WASI_ENTRY_POINT {
                                        debug!("Has entry point: {}", export.name);
                                        new.has_start = Some(true);
                                    }

                                    if export.name == constants::VRRB_WASM_MAGIC
                                        || export.name == constants::VRRB_WASM_VERSION
                                    {
                                        debug!("Has VRRB symbol: {}", export.name);
                                        new.has_vrrb = Some(true);
                                    }

                                    // XXX: in the future, keep the exports list
                                    // too
                                }
                            },
                            Payload::ImportSection(s) => {
                                for import in s {
                                    let import = import.expect("Import section is malformed");

                                    debug!("Import: {:?}", import);

                                    if imports.get(import.module).is_none() {
                                        imports.insert(import.module.to_string(), Vec::new());
                                    }

                                    imports
                                        .get_mut(import.module)
                                        .expect("Vector creation failed.")
                                        .push(import.name.to_string());

                                    if import.module == constants::WASI_NAMESPACE_PREVIEW1
                                        || import.module == constants::WASI_NAMESPACE_UNSTABLE
                                    {
                                        new.is_wasi = Some(true);
                                    }

                                    if import.module == constants::WASIX_NAMESPACE_32V1
                                        || import.module == constants::WASIX_NAMESPACE_64V1
                                    {
                                        new.is_wasi = Some(true);
                                        new.is_wasix = Some(true);
                                    }

                                    if import.module == constants::JAVY_NAMESPACE_QUICKJS {
                                        new.needs_javy = Some(true);
                                    }
                                }
                            },
                            _other => {},
                        }
                    },
                    Err(e) => return Err(e.into()),
                }
            }
        }

        new.imports = Some(imports);

        Ok(new)
    }
}
