use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const IO_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "read_line",
        StdFunctionKind::IoReadLine,
        StdArity::Exact(0),
        &[],
        StdType::Any,
    ),
    std_function(
        "read_all",
        StdFunctionKind::IoReadAll,
        StdArity::Exact(0),
        &[],
        StdType::String,
    ),
    std_function(
        "read_bytes",
        StdFunctionKind::IoReadBytes,
        StdArity::Exact(0),
        &[],
        StdType::List,
    ),
    std_function(
        "write",
        StdFunctionKind::IoWrite,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "write_err",
        StdFunctionKind::IoWriteErr,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "flush",
        StdFunctionKind::IoFlush,
        StdArity::Exact(0),
        &[],
        StdType::Void,
    ),
];

pub const MODULE: StdModule =
    std_module!("std.io", "Standard input/output helpers.", IO_FUNCTIONS,);
