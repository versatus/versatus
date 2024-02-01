//! This contains various abstractions and implementations for being able to provide a collection
//! of compute runtimes for different purposes and using different open source components.

mod kontain; // The Kontain unikernel container runtime.
mod kontain_wasm; // versatus-wasm with the Kontain container runtime
mod oci; // Common code for managing OCI-compatible container runtimes
mod oci_runc; // OCI reference container runtime.
mod oci_wasm; // versatus-wasm with the OCI reference container runtime
pub mod runtime; // core crate entrypoint
mod youki; // youki container runtime

#[cfg(test)]
mod tests;
