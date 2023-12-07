use std::error::Error;
use wasmer::RuntimeError;
use wasmer::{CompileError, ExportError, FrameInfo, InstantiationError};
use wasmer_vm::TrapCode;
use wasmer_wasix::{WasiError, WasiRuntimeError};

#[derive(thiserror::Error, Debug)]
pub enum WasmRuntimeError {
    #[error(
        "Encountered runtime error:
reason: {:?}
msg:    {}
trace:  {:?}
origin: {:?}",
        reason,
        msg,
        trace,
        origin
    )]
    RuntimeError {
        reason: TrapCode,
        msg: String,
        trace: Vec<FrameInfo>,
        origin: Option<String>,
    },

    #[error(
        "Encountered runtime error:
msg:    {}
trace:  {:?}
origin: {:?}",
        msg,
        trace,
        origin
    )]
    RuntimeErrorLossy {
        msg: String,
        trace: Vec<FrameInfo>,
        origin: Option<String>,
    },

    #[error(transparent)]
    CompileError(#[from] CompileError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    WasiRuntimeError(#[from] WasiRuntimeError),

    #[error(transparent)]
    WasiError(#[from] WasiError),

    #[error(transparent)]
    InstantiationError(#[from] InstantiationError),

    #[error(transparent)]
    ExportError(#[from] ExportError),
}

impl WasmRuntimeError {
    pub fn origin(&self) -> Option<String> {
        match self {
            Self::RuntimeError { origin, .. } => origin.clone(),
            Self::RuntimeErrorLossy { origin, .. } => origin.clone(),
            _ => None,
        }
    }
    pub fn reason(&self) -> Option<TrapCode> {
        if let Self::RuntimeError { reason, .. } = self {
            return Some(reason.clone());
        }
        None
    }
    pub fn inst_err(&self) -> Option<String> {
        if let WasmRuntimeError::InstantiationError(inst_err) = self {
            match inst_err {
                InstantiationError::Link(link_err) => match link_err {
                    wasmer::LinkError::Resource(s) => Some(s.clone()),
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }
}

impl From<RuntimeError> for WasmRuntimeError {
    fn from(value: RuntimeError) -> Self {
        if let Some(reason) = value.clone().to_trap() {
            Self::RuntimeError {
                reason,
                msg: value.message(),
                trace: value.trace().to_owned(),
                origin: value.source().and_then(|err| Some(format!("{err:?}"))),
            }
        } else {
            Self::RuntimeErrorLossy {
                msg: value.message(),
                trace: value.trace().to_owned(),
                origin: value.source().and_then(|err| Some(format!("{err:?}"))),
            }
        }
    }
}
