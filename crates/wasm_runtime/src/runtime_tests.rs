use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};
use wasmer::{Cranelift, Target};
use wasmer_vm::TrapCode;

use crate::{
    errors::WasmRuntimeError,
    metering::{cost_function, MeteringConfig},
    wasm_runtime::WasmRuntime,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestInput {
    version: i32,
    tx_id: String,
    last_block_time: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestOutput {
    stdin: TestInput,
    args: Vec<String>,
    env: HashMap<String, String>,
}

// Constants used by tests below
const TEST_VERSION: i32 = 5432;
const TEST_TX_ID: &str = "81b067ac-8693-483a-8354-d7de15ab6f2c";
const TEST_LAST_BLOCK_TIME: i64 = 1689897402;
const TEST_RETURN_FAIL: &str = "RETURN_FAIL";
const VRRB_CONTRACT_NAME: &str = "vrrb-contract"; //argv[0] for smart contracts
const TEST_SPENDING_LIMIT: u64 = 1000000;

fn create_test_wasm_runtime(
    target: &Target,
    wasm_bytes: &[u8],
) -> crate::wasm_runtime::RuntimeResult<WasmRuntime> {
    let metering_config = MeteringConfig::new(TEST_SPENDING_LIMIT, cost_function);
    WasmRuntime::new::<Cranelift>(target, wasm_bytes, metering_config)
}

/// This test checks that the stuff we send via stdin is available as part of
/// the struct on stdout, per the wasm_test.wasm functionality. It shows we're
/// able to properly set up the stdin and stdout pipes.
/// XXX: This test currently fails because multiline JSON (serde output) blocks
/// forever waiting for EOF on stdin in the wasm_test.wasm module, which will
/// never come. This needs to be rectified and this test pass before going live.
#[test]
#[ignore]
fn test_multiline_input() {
    let wasm_bytes = std::fs::read("test_data/wasm_test.wasm").unwrap();
    let inputs = TestInput {
        version: TEST_VERSION,
        tx_id: TEST_TX_ID.to_string(),
        last_block_time: TEST_LAST_BLOCK_TIME,
    };
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&serde_json::to_vec(&inputs).unwrap());
    runtime.execute().unwrap();

    let out: TestOutput = serde_json::from_str(&runtime.stdout()).unwrap();

    assert_eq!(out.stdin.version, TEST_VERSION);
    assert_eq!(out.stdin.tx_id, TEST_TX_ID);
    assert_eq!(out.stdin.last_block_time, TEST_LAST_BLOCK_TIME);
}

/// This test checks for inputs passed on, on the output of the test module
/// output object.
#[test]
fn test_single_line_input() {
    let wasm_bytes = std::fs::read("test_data/wasm_test.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data);
    runtime.execute().unwrap();

    let out: TestOutput = serde_json::from_str(&runtime.stdout()).unwrap();

    assert_eq!(out.stdin.version, TEST_VERSION);
    assert_eq!(out.stdin.tx_id, TEST_TX_ID);
    assert_eq!(out.stdin.last_block_time, TEST_LAST_BLOCK_TIME);
}

/// This test checks for correctness of command line arguments in the WASM
/// object's output as having been passed through untouched.
#[test]
fn test_command_line_args() {
    let wasm_bytes = std::fs::read("test_data/wasm_test.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let args: Vec<String> = vec![
        "all".to_string(),
        "your".to_string(),
        "WASM".to_string(),
        "are".to_string(),
        "belong".to_string(),
        "to".to_string(),
        "us".to_string(),
    ];
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data)
        .args(&args);
    runtime.execute().unwrap();

    let out: TestOutput = serde_json::from_str(&runtime.stdout()).unwrap();

    assert_eq!(out.args[0], VRRB_CONTRACT_NAME);
    assert_eq!(out.args[1..], args);
}

/// This test checks for correctness of environment variables passed to the WASM
/// object as having been passed through untouched.
#[test]
fn test_environment_vars() {
    let wasm_bytes = std::fs::read("test_data/wasm_test.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let mut wasm_env: HashMap<String, String> = HashMap::new();
    wasm_env.insert("GUMBY".to_string(), "gumby".to_string());
    wasm_env.insert("POKEY".to_string(), "pokey".to_string());
    wasm_env.insert("PRICKLE".to_string(), "prickle".to_string());
    wasm_env.insert("GOO".to_string(), "goo".to_string());
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data)
        .env(&wasm_env);
    runtime.execute().unwrap();

    let out: TestOutput = serde_json::from_str(&runtime.stdout()).unwrap();

    assert_eq!(out.env, wasm_env);
}
///
/// This check tests that we correctly report when a WASM module fails to
/// execute. This is done by setting a special variable that the WASM test
/// module uses to trigger failure.
#[test]
#[should_panic]
fn test_failed_execution() {
    let wasm_bytes = std::fs::read("test_data/wasm_test.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let mut wasm_env: HashMap<String, String> = HashMap::new();
    wasm_env.insert(TEST_RETURN_FAIL.to_string(), "true".to_string());
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data)
        .env(&wasm_env);
    runtime.execute().unwrap();

    let _out: TestOutput = serde_json::from_str(&runtime.stdout()).unwrap();
}

/// This test checks for the return of a mock instance of infinite recursion.
#[test]
fn test_mem_alloc() {
    let wasm_bytes = std::fs::read("test_data/should_panic/mem_alloc.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data);
    let res = runtime.execute();
    assert_eq!(res.err().unwrap().inst_err(), Some("Failed to create memory: A user-defined error occurred: Minimum exceeds the allowed memory limit".to_string()));
}

// This test checks for the return of a mock instance of stack overflow.
#[test]
fn test_stack_overflow() {
    let wasm_bytes = std::fs::read("test_data/should_panic/stack_overflow.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data);
    let res = runtime.execute();
    assert_eq!(res.err().unwrap().reason(), Some(TrapCode::StackOverflow));
}

// This test checks for the return of a non-existent file attempting to be read.
#[test]
fn test_file_not_found() {
    let wasm_bytes = std::fs::read("test_data/should_panic/file_not_found.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data);
    let res = runtime.execute();
    assert_eq!(
        res.err().unwrap().reason(),
        Some(TrapCode::UnreachableCodeReached)
    );
}

// This test checks for the return of i32 integer using std::process::exit().
#[test]
fn test_process_exit() {
    let wasm_bytes = std::fs::read("test_data/should_panic/process_exit.wasm").unwrap();
    let json_data = std::fs::read("test_data/wasm_test_oneline.json").unwrap();
    let target = Target::default();
    let mut runtime = create_test_wasm_runtime(&target, &wasm_bytes)
        .unwrap()
        .stdin(&json_data);
    let res = runtime.execute();
    assert_eq!(
        res.err().unwrap().origin(),
        Some("Exit(ExitCode::2147483647)".to_string())
    );
}
