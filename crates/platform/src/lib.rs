//! This crate contains a bunch of platform-related functionality. Specifically
//! the Linux operating system for the moment, but could be extended to support
//! other operating systems at least in part in the future.

pub mod platform_stats;
pub mod services;

#[allow(non_camel_case_types)]
pub enum Machine {
    x86_64,
    AArch64,
    Unsupported,
}
impl<'a> From<&'a str> for Machine {
    fn from(value: &'a str) -> Self {
        match value {
            "x86_64" => Self::x86_64,
            "aarch64" => Self::AArch64,
            _ => Self::Unsupported,
        }
    }
}

/// POSIX utsname header
pub struct PlatformCapabilities(uname_rs::Uname);
impl PlatformCapabilities {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self(uname_rs::Uname::new()?))
    }

    /// Name of this implementation of the operating system.
    pub fn sysname(&self) -> String {
        self.0.sysname.to_owned()
    }

    /// Name of the hardware type on which the system is running.
    pub fn machine(&self) -> String {
        self.0.machine.to_owned()
    }
}

#[test]
fn test_sysname() {
    dbg!(PlatformCapabilities::new().unwrap().sysname());
}
#[test]
fn test_machine() {
    dbg!(PlatformCapabilities::new().unwrap().machine());
}
