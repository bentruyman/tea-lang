use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const PROCESS_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "run",
        kind: StdFunctionKind::ProcessRun,
        arity: StdArity::Range {
            min: 1,
            max: Some(5),
        },
        params: &[
            StdType::String,
            StdType::List,
            StdType::Dict,
            StdType::String,
            StdType::String,
        ],
        return_type: StdType::Struct,
    },
    StdFunction {
        name: "spawn",
        kind: StdFunctionKind::ProcessSpawn,
        arity: StdArity::Range {
            min: 1,
            max: Some(4),
        },
        params: &[
            StdType::String,
            StdType::List,
            StdType::Dict,
            StdType::String,
        ],
        return_type: StdType::Int,
    },
    StdFunction {
        name: "wait",
        kind: StdFunctionKind::ProcessWait,
        arity: StdArity::Exact(1),
        params: &[StdType::Int],
        return_type: StdType::Struct,
    },
    StdFunction {
        name: "kill",
        kind: StdFunctionKind::ProcessKill,
        arity: StdArity::Exact(1),
        params: &[StdType::Int],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "read_stdout",
        kind: StdFunctionKind::ProcessReadStdout,
        arity: StdArity::Range {
            min: 1,
            max: Some(2),
        },
        params: &[StdType::Int, StdType::Int],
        return_type: StdType::String,
    },
    StdFunction {
        name: "read_stderr",
        kind: StdFunctionKind::ProcessReadStderr,
        arity: StdArity::Range {
            min: 1,
            max: Some(2),
        },
        params: &[StdType::Int, StdType::Int],
        return_type: StdType::String,
    },
    StdFunction {
        name: "write_stdin",
        kind: StdFunctionKind::ProcessWriteStdin,
        arity: StdArity::Exact(2),
        params: &[StdType::Int, StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "close_stdin",
        kind: StdFunctionKind::ProcessCloseStdin,
        arity: StdArity::Exact(1),
        params: &[StdType::Int],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "close",
        kind: StdFunctionKind::ProcessClose,
        arity: StdArity::Exact(1),
        params: &[StdType::Int],
        return_type: StdType::Nil,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.process",
    functions: PROCESS_FUNCTIONS,
};
