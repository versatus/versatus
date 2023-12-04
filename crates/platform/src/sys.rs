use crate::error::{ConversionError, PlatformError};
use nix::sys::utsname::UtsName;
use std::{ffi::OsStr, fmt::Debug};

const MACHINE_ARCH: &str = "MachineArchitecture";
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum MachineArchitecture {
    x86_64,
    Aarch64,
    Riscv,
}
impl<'a> TryFrom<&'a OsStr> for MachineArchitecture {
    type Error = PlatformError;
    fn try_from(value: &'a OsStr) -> anyhow::Result<Self, Self::Error> {
        let err = ConversionError {
            from: value,
            into: MACHINE_ARCH,
            msg: "unable to convert system architecture into concrete type",
        };
        let res = match value.to_ascii_lowercase().to_str().ok_or(err.clone())? {
            "x86_64" => Self::x86_64,
            "aarch64" | "arm64" => Self::Aarch64,
            "riscv" => Self::Riscv,
            _ => return Err(err.into()),
        };
        Ok(res)
    }
}
impl<'a> TryFrom<&'a UtsName> for MachineArchitecture {
    type Error = PlatformError;
    fn try_from(value: &'a UtsName) -> anyhow::Result<Self, Self::Error> {
        MachineArchitecture::try_from(value.machine())
    }
}

const SYSNAME: &str = "Sysname";
pub enum Sysname {
    Linux,
    Darwin,
}
impl<'a> TryFrom<&'a OsStr> for Sysname {
    type Error = PlatformError;
    fn try_from(value: &'a OsStr) -> anyhow::Result<Self, Self::Error> {
        let err = ConversionError {
            from: value,
            into: SYSNAME,
            msg: "unable to convert system name into concrete type",
        };
        let res = match value.to_ascii_lowercase().to_str().ok_or(err.clone())? {
            "linux" => Self::Linux,
            "darwin" => Self::Darwin,
            _ => return Err(err.into()),
        };
        Ok(res)
    }
}
impl<'a> TryFrom<&'a UtsName> for Sysname {
    type Error = PlatformError;
    fn try_from(value: &'a UtsName) -> anyhow::Result<Self, Self::Error> {
        Sysname::try_from(value.sysname())
    }
}

#[test]
fn test_sysname() {
    dbg!(nix::sys::utsname::uname().unwrap().sysname());
}
#[test]
fn test_machine() {
    dbg!(nix::sys::utsname::uname().unwrap().machine());
}
