//! This module defines the ComputeRuntime trait adhered to by Versatus compute runtimes.
use anyhow::Result;
use bitmask_enum::bitmask;

#[bitmask]
pub enum ComputeRuntimeCapabilities {
    Wasm,
    Native,
    Python,
}

pub trait ComputeRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities;
    fn domainname() -> &'static str;
    fn setup(&self, job_id: &str, runtime_path: &str) -> Result<()>;
}
