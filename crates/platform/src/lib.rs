//! This crate contains a bunch of platform-related functionality. Specifically
//! the Linux operating system for the moment, but could be extended to support
//! other operating systems at least in part in the future.

pub mod error;
pub mod platform_stats;
pub mod services;
pub mod sys;

pub use nix::sys::utsname::*;
