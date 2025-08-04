use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const IO_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "read_line",
        kind: StdFunctionKind::IoReadLine,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::Any,
    },
    StdFunction {
        name: "read_all",
        kind: StdFunctionKind::IoReadAll,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::String,
    },
    StdFunction {
        name: "read_bytes",
        kind: StdFunctionKind::IoReadBytes,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::List,
    },
    StdFunction {
        name: "write",
        kind: StdFunctionKind::IoWrite,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "write_err",
        kind: StdFunctionKind::IoWriteErr,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "flush",
        kind: StdFunctionKind::IoFlush,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::Nil,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.io",
    functions: IO_FUNCTIONS,
};
