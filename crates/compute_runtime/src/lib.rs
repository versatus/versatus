//! This contains various abstractions and implementations for being able to provide a collection
//! of compute runtimes for different purposes and using different open source components.

mod kontain;
mod kontain_wasm;
mod oci;
mod oci_runc;
mod runtime;
mod youki;

#[cfg(test)]
mod tests;
