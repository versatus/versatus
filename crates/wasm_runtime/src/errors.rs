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

    /// Inherits the error value propagated when creation of a new
    /// executable instance of a WASM module fails.
    #[error(transparent)]
    InstantiationError(Box<InstantiationError>),

    #[error(transparent)]
    ExportError(#[from] ExportError),

    #[error("failed to build wasm runtime module: {0}")]
    ModuleBuildError(String),
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
            return Some(*reason);
        }
        None
    }
    pub fn inst_err(&self) -> Option<String> {
        if let WasmRuntimeError::InstantiationError(boxed_err) = self {
            if let InstantiationError::Link(wasmer::LinkError::Resource(s)) = &**boxed_err {
                Some(s.to_owned())
            } else {
                None
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
                origin: value.source().map(|err| format!("{err:?}")),
            }
        } else {
            Self::RuntimeErrorLossy {
                msg: value.message(),
                trace: value.trace().to_owned(),
                origin: value.source().map(|err| format!("{err:?}")),
            }
        }
    }
}

impl From<InstantiationError> for WasmRuntimeError {
    fn from(value: InstantiationError) -> Self {
        Self::InstantiationError(Box::new(value))
    }
}
