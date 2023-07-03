//! Web Assembly loader and validator
//!
//! Provides some basic functionality to load Web Assembly bytes (or WAT
//! strings) from files or other locations and perform some basic VRRB sanity
//! checking or inspection of the loaded module(s).

pub mod wasm_loader {
    use std::collections::HashMap;

    //use log::{debug, info};
    use telemetry::log::{debug,info};
    use wasmer::wat2wasm;
    use wasmparser::{Parser, Payload};

    // Constants
    /// The primary namespace used by WASI implementations.
    const WASI_NAMESPACE_PREVIEW1: &str = "wasi_snapshot_preview1";
    /// The namespace used by Javy for JS->WASM.
    const JAVY_NAMESPACE_QUICKJS: &str = "javy_quickjs_provider_v1";
    /// The magic bytes at the start of every WASM v1 object.
    const WASM_MAGIC: &[u8; 4] = &[0x00, 0x61, 0x73, 0x6d];
    /// The offset from which to have the WASM parser start. Currently always 0.
    const WASM_PARSE_OFFSET: u64 = 0;
    /// Default entry point for WASM modules (think main()).
    const WASI_ENTRY_POINT: &str = "_start";
    /// A VRRB-specific magic string potentially exported by modules
    const VRRB_WASM_MAGIC: &str = "_vrrb_abi_magic";
    /// A VRRB-specific version number potentially exported by modules
    const VRRB_WASM_VERSION: &str = "_vrrb_abi_version";

    /// A struct to represent some loaded and parsed WASM.
    #[derive(Default, Debug)]
    pub struct WasmLoader {
        /// The bytes of WASM themselves, potentially having been translated
        /// from WAT.
        wasm_bytes: Vec<u8>,
        /// Represents the size of the WASM binary.
        wasm_size: usize,
        /// True is this module uses WASI interfaces. Currently tested by the
        /// presence of the [WASI_NAMESPACE_PREVIEW1] namespace.
        is_wasi: bool,
        /// True if this WASM module was compiled with Javy and needs the
        /// quickjs library.
        is_javy: bool,
        /// True if this WASM was loaded as WAT and translated to WASM.
        from_wat: bool,
        /// True if this WASM module exports a _start symbol
        has_start: bool,
        /// True if this WASM module exports any of the VRRB magic symbols
        has_vrrb: bool,
        /// A HashMap where the key is a string representing the namespace, and
        /// the value is a set of strings representing symbols this
        /// module expects to be present. Used in determining
        /// dependencies, etc.
        imports: HashMap<String, Vec<String>>,
    }

    impl WasmLoader {
        /// Returns a struct representing some WASM as loaded from a file and
        /// parsed.
        ///
        /// # Arguments
        ///
        /// * `filename` - A string path to the WASM file on the filesystem.
        ///
        /// # Examples
        ///
        /// ```
        /// use wasm_loader::wasm_loader::WasmLoader;
        /// let res = WasmLoader::from_file("test_data/simple-javy.wasm");
        /// ```
        pub fn from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let mut is_wasi = false;
            let mut from_wat = false;
            let mut is_javy = false;
            let mut has_start = false;
            let mut has_vrrb = false;

            let mut bytes = std::fs::read(filename)?;

            // If it's not WASM (checking the magic first 4 bytes), try converting from WAT first.
            let byte_header = &bytes[0..4];
            if byte_header != WASM_MAGIC {
                // It's not WASM, so let's try compiling it from WAT
                info!("{} isn't WASM, trying to compile from WAT", filename);
                debug!("{:?} != {:?}", byte_header, WASM_MAGIC);
                bytes = wat2wasm(&bytes)?.into_owned();
                from_wat = true;
            }

            // Map of namespaces and the symbols this module expects to import.
            let mut imports_map: HashMap<String, Vec<String>> = HashMap::new();
            // Parse out WASM symbols
            for payload in Parser::new(WASM_PARSE_OFFSET).parse_all(&bytes) {
                match payload {
                    Ok(p) => {
                        match p {
                            Payload::Version { .. } => {
                                debug!("WASM");
                            },
                            Payload::ExportSection(s) => {
                                for export in s {
                                    let export = export?;
                                    //debug!("Export {:?}", export);

                                    // Check whether this is an exported entry point
                                    if export.name == WASI_ENTRY_POINT {
                                        has_start = true;
                                    }
                                    // Or whether it's using any VRRB magic. This should check more
                                    // than just the existence of the global export, and should
                                    // later check the magic number and version as we define those
                                    // interfaces more. XXX.
                                    if export.name == VRRB_WASM_MAGIC
                                        || export.name == VRRB_WASM_VERSION
                                    {
                                        has_vrrb = true;
                                    }
                                }
                            },
                            Payload::ImportSection(s) => {
                                for import in s {
                                    // Create a new vector if one doesn't exist
                                    let import = import?;
                                    if imports_map.get(import.module).is_none() {
                                        imports_map.insert(import.module.to_string(), Vec::new());
                                    }

                                    imports_map
                                        .get_mut(import.module)
                                        .expect("Vector creation failed.")
                                        .push(import.name.to_string());

                                    // Remember having seen certain import namespaces.
                                    // WASI preview1? Note, we should add the WASIX namespace
                                    // later.
                                    if import.module == WASI_NAMESPACE_PREVIEW1 {
                                        is_wasi = true;
                                    }
                                    // Javy's dependency namespace?
                                    if import.module == JAVY_NAMESPACE_QUICKJS {
                                        is_javy = true;
                                    }

                                    debug!("Import {:?}", import);
                                }
                            },
                            _other => {
                                //debug!("Other: {:?}", _other)
                            },
                        }
                    },
                    Err(e) => {
                        return Err(Box::new(e));
                    },
                }
            }

            debug!("Imports by namespace: {:?}", imports_map);

            Ok(Self {
                wasm_bytes: bytes.to_owned(),
                wasm_size: bytes.len(),
                is_wasi,
                is_javy,
                from_wat,
                has_start,
                has_vrrb,
                imports: imports_map.to_owned(),
            })
        }
    }

    #[cfg(test)]
    mod tests {
        //use test_log::test;

        use crate::wasm_loader::WasmLoader;

        #[test]
        fn file_not_found() {
            let res = WasmLoader::from_file("test_data/notfound.wasm");
            // show that we barf if the file doesn't exist
            assert!(res.is_err());
        }

        #[test]
        fn file_load_wasm() {
            let res = WasmLoader::from_file("test_data/simple.wasm");

            match res {
                Ok(r) => {
                    assert_ne!(r.wasm_size, 0);
                },
                Err(_) => assert!(res.is_ok()),
            }
        }

        #[test]
        fn file_load_wat() {
            let res = WasmLoader::from_file("test_data/simple.wat");
            match res {
                Ok(r) => assert_ne!(r.wasm_size, 0),
                Err(_) => assert!(res.is_ok()),
            }
        }

        #[test]
        fn file_check_from_wat() {
            let res = WasmLoader::from_file("test_data/simple.wat");
            // Check that we remembered compiling it from WAT
            assert!(
                res.unwrap().from_wat,
                "Should have known we compiled WASM from WAT"
            );
        }

        #[test]
        fn file_load_wasi() {
            let res = WasmLoader::from_file("test_data/simple.wasi");
            match res {
                Ok(r) => assert_ne!(r.wasm_size, 0),
                Err(_) => assert!(res.is_ok()),
            }
        }

        #[test]
        fn file_check_wasi_symbols() {
            let res = WasmLoader::from_file("test_data/simple.wasi");
            // We're expecting this to look like WASI if the parser works.
            assert!(res.unwrap().is_wasi, "Should have detected WASI symbols");
        }

        #[test]
        fn file_check_vrrb_symbols() {
            let res = WasmLoader::from_file("test_data/simple.wasi");
            // This module also has some preliminary VRRB-related symbols. We should have
            // seen them.
            assert!(res.unwrap().has_vrrb, "Should have detected VRRB symbols");
        }

        #[test]
        fn file_check_start_symbol() {
            let res = WasmLoader::from_file("test_data/simple.wasi");
            // This module exports a _start symbol (eg, main()).
            assert!(
                res.unwrap().has_start,
                "Should have detected _start symbol export"
            );
        }

        #[test]
        fn file_check_javy_symbols() {
            let res = WasmLoader::from_file("test_data/simple-javy.wasm");
            // Check that we can discover that this module needs the Javy runtime imported.
            assert!(res.unwrap().is_javy, "Should have detected Javy symbols");
        }
    }
}
