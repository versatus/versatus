use crate::error::{ConversionError, PlatformError};
use std::fmt::Debug;

const MACHINE_ARCH: &str = "MachineArchitecture";
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum MachineArchitecture {
    x86_64,
    Aarch64,
    Riscv,
}
impl TryFrom<String> for MachineArchitecture {
    type Error = PlatformError;
    fn try_from(value: String) -> anyhow::Result<Self, Self::Error> {
        let res = match value.to_lowercase().as_str() {
            "x86_64" => Self::x86_64,
            "aarch64" | "i.local" | "arm64" => Self::Aarch64,
            "riscv" => Self::Riscv,
            _ => {
                return Err(ConversionError {
                    from: value,
                    into: MACHINE_ARCH,
                    msg: "unable to convert system architecture into concrete type",
                }
                .into())
            }
        };
        Ok(res)
    }
}
impl<'a> TryFrom<&'a Utsname> for MachineArchitecture {
    type Error = PlatformError;
    fn try_from(value: &'a Utsname) -> anyhow::Result<Self, Self::Error> {
        MachineArchitecture::try_from(value.machine())
    }
}

const SYSNAME: &str = "Sysname";
pub enum Sysname {
    Linux,
    Darwin,
}
impl TryFrom<String> for Sysname {
    type Error = PlatformError;
    fn try_from(value: String) -> anyhow::Result<Self, Self::Error> {
        let res = match value.to_lowercase().as_str() {
            "linux" => Self::Linux,
            "darwin" => Self::Darwin,
            _ => {
                return Err(ConversionError {
                    from: value,
                    into: SYSNAME,
                    msg: "unable to convert system name into concrete type",
                }
                .into())
            }
        };
        Ok(res)
    }
}
impl<'a> TryFrom<&'a Utsname> for Sysname {
    type Error = PlatformError;
    fn try_from(value: &'a Utsname) -> anyhow::Result<Self, Self::Error> {
        Sysname::try_from(value.sysname())
    }
}

/// POSIX utsname header
pub struct Utsname(uname_rs::Uname);
impl Utsname {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(uname_rs::Uname::new().map_err(PlatformError::POSIX)?))
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
impl std::fmt::Debug for Utsname {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Utsname:
    sysname:    {}
    nodename:   {}
    release:    {}
    version:    {}
    machine:    {}
    domainname: {}
",
            self.0.sysname,
            self.0.nodename,
            self.0.release,
            self.0.version,
            self.0.machine,
            self.0.domainname,
        )
    }
}

#[test]
fn test_sysname() {
    dbg!(Utsname::new().unwrap().sysname());
}
#[test]
fn test_machine() {
    dbg!(Utsname::new().unwrap().machine());
}
