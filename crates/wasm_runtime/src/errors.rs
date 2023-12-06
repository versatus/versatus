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
