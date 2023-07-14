// Constants
/// The initial public version of the WASI interface.
pub const WASI_NAMESPACE_UNSTABLE: &str = "wasi_unstable";
/// The primary namespace used by WASI implementations.
pub const WASI_NAMESPACE_PREVIEW1: &str = "wasi_snapshot_preview1";
/// The WASIX soon-to-be standard for 32bit WASIX
pub const WASIX_NAMESPACE_32V1: &str = "wasix_32v1";
/// The WASIX soon-to-be standard for 64bit WASIX
pub const WASIX_NAMESPACE_64V1: &str = "wasix_64v1";

/// The namespace used by Javy for JS->WASM.
pub const JAVY_NAMESPACE_QUICKJS: &str = "javy_quickjs_provider_v1";

/// The magic bytes at the start of every WASM v1 object.
pub const WASM_MAGIC: &[u8; 4] = &[0x00, 0x61, 0x73, 0x6d];
/// The offset from which to have the WASM parser start. Currently always 0.
pub const WASM_PARSE_OFFSET: u64 = 0;
/// Default entry point for WASM modules (think main()).
pub const WASI_ENTRY_POINT: &str = "_start";
/// A VRRB-specific magic string potentially exported by modules
pub const VRRB_WASM_MAGIC: &str = "_vrrb_abi_magic";
/// A VRRB-specific version number potentially exported by modules
pub const VRRB_WASM_VERSION: &str = "_vrrb_abi_version";
