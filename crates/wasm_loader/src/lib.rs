mod constants;
pub mod wasm_loader;

#[cfg(test)]
mod loader_tests {
    use telemetry::log::debug;
    use test_log::test;

    use crate::wasm_loader::WasmLoaderBuilder;

    // constants to some precompiled WASM modules to aid in some basic testing.
    // A module containing some WASI symbols and some VRRB symbols
    const SIMPLE_WASI_TEST_MODULE: &str = "test_data/simple.wasi";
    // A module with a Javy dependency
    const SIMPLE_JAVY_TEST_MODULE: &str = "test_data/simple-javy.wasm";
    // A WASM module represented as Web Assembly Text (WAT) to be assembled/compiled
    // to binary
    const SIMPLE_WAT_TEST_MODULE: &str = "test_data/simple.wat";

    #[test]
    fn builder_wasm_load() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(SIMPLE_WASI_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        debug!("w: {:02x?}", w);
        assert!(w.is_ok());
    }

    #[test]
    #[should_panic]
    fn builder_wasm_bad_data() {
        // valid-length WASM header with almost-valid values
        let data = vec![0x00, 0x61, 0x73, 0x73];
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(data)
            .parse()
            .expect("Invalid WASM data")
            .build();
        assert!(w.is_err());
    }

    #[test]
    fn builder_load_wat() {
        let w = WasmLoaderBuilder::default()
            .wat_text(std::fs::read(SIMPLE_WAT_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        if let Ok(wasm) = w {
            debug!("w: {:02x?}", wasm);
            assert!(wasm.from_wat);
        }
    }

    #[test]
    fn builder_load_no_wasix() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(SIMPLE_WASI_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        if let Ok(wasm) = w {
            debug!("w: {:02x?}", wasm);
            assert!(!wasm.is_wasix, "Shouldn't have found any WASIX symbols");
        }
    }

    //XXX: Test data to generate for additional test cases:
    //  - Binary with WASIX symbols (Rust?)
    //  - 64bit as well as 32bit (Rust?)

    #[test]
    fn builder_load_wasi() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(SIMPLE_WASI_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        if let Ok(wasm) = w {
            debug!("w: {:02x?}", wasm);
            assert!(wasm.is_wasi, "Didn't find WASI namespace(s)");
        }
    }

    #[test]
    fn builder_check_vrrb_symbols() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(SIMPLE_WASI_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        if let Ok(wasm) = w {
            debug!("w: {:02x?}", wasm);
            assert!(wasm.has_vrrb, "Didn't find VRRB symbols");
        }
    }

    #[test]
    fn builder_check_start_symbol() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(SIMPLE_WASI_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        if let Ok(wasm) = w {
            debug!("w: {:02x?}", wasm);
            assert!(wasm.has_start, "Didn't find _start symbol");
        }
    }

    #[test]
    fn builder_check_javy_symbols() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read(SIMPLE_JAVY_TEST_MODULE).unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        if let Ok(wasm) = w {
            debug!("w: {:02x?}", wasm);
            assert!(wasm.needs_javy, "Didn't find expected Javy dependency");
        }
    }
}
