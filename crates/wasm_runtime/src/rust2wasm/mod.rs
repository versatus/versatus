//! Rust programs that should compile to the wasm32-wasi target
//! and be stored in the test_data directory.
pub mod file_not_found;
pub mod infinite_recursion;
pub mod process_exit;
pub mod trigger_oom;
