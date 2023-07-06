pub mod wasm_loader;

#[cfg(test)]
mod loader_tests {
    use crate::wasm_loader::WasmLoaderBuilder;
    use telemetry::log::debug;
    use test_log::test;

    #[test]
    fn builder_wasm_load() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read("test_data/simple.wasi").unwrap())
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
            .wat_text(std::fs::read("test_data/simple.wat").unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        match w {
            Ok(wasm) => {
                debug!("w: {:02x?}", wasm);
                assert!(wasm.from_wat);
            },
            Err(_) => {}, // Handled by is_ok() assert above
        }
    }

    #[test]
    fn builder_load_no_wasix() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read("test_data/simple.wasi").unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        match w {
            Ok(wasm) => {
                debug!("w: {:02x?}", wasm);
                assert!(!wasm.is_wasix, "Shouldn't have found any WASIX symbols");
            },
            Err(_) => {},
        }
    }

    //XXX: Test data to generate for additional test cases:
    //  - Binary with WASIX symbols (Rust?)
    //  - 64bit as well as 32bit (Rust?)

    #[test]
    fn builder_load_wasi() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read("test_data/simple.wasi").unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        match w {
            Ok(wasm) => {
                debug!("w: {:02x?}", wasm);
                assert!(wasm.is_wasi, "Didn't find WASI namespace(s)");
            },
            Err(_) => {},
        }
    }

    #[test]
    fn builder_check_vrrb_symbols() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read("test_data/simple.wasi").unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        match w {
            Ok(wasm) => {
                debug!("w: {:02x?}", wasm);
                assert!(wasm.has_vrrb, "Didn't find VRRB symbols");
            },
            Err(_) => {},
        }
    }

    #[test]
    fn builder_check_start_symbol() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read("test_data/simple.wasi").unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        match w {
            Ok(wasm) => {
                debug!("w: {:02x?}", wasm);
                assert!(wasm.has_start, "Didn't find _start symbol");
            },
            Err(_) => {},
        }
    }

    #[test]
    fn builder_check_javy_symbols() {
        let w = WasmLoaderBuilder::default()
            .wasm_bytes(std::fs::read("test_data/simple-javy.wasm").unwrap())
            .parse()
            .unwrap()
            .build();
        assert!(w.is_ok());
        match w {
            Ok(wasm) => {
                debug!("w: {:02x?}", wasm);
                assert!(wasm.needs_javy, "Didn't find expected Javy dependency");
            },
            Err(_) => {},
        }
    }
}
