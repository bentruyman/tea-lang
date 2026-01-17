use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const PROCESS_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "run",
        StdFunctionKind::ProcessRun,
        StdArity::Range {
            min: 1,
            max: Some(5),
        },
        &[
            StdType::String,
            StdType::List,
            StdType::Dict,
            StdType::String,
            StdType::String,
        ],
        StdType::Struct,
    ),
    std_function(
        "spawn",
        StdFunctionKind::ProcessSpawn,
        StdArity::Range {
            min: 1,
            max: Some(4),
        },
        &[
            StdType::String,
            StdType::List,
            StdType::Dict,
            StdType::String,
        ],
        StdType::Int,
    ),
    std_function(
        "wait",
        StdFunctionKind::ProcessWait,
        StdArity::Exact(1),
        &[StdType::Int],
        StdType::Struct,
    ),
    std_function(
        "kill",
        StdFunctionKind::ProcessKill,
        StdArity::Exact(1),
        &[StdType::Int],
        StdType::Bool,
    ),
    std_function(
        "read_stdout",
        StdFunctionKind::ProcessReadStdout,
        StdArity::Range {
            min: 1,
            max: Some(2),
        },
        &[StdType::Int, StdType::Int],
        StdType::String,
    ),
    std_function(
        "read_stderr",
        StdFunctionKind::ProcessReadStderr,
        StdArity::Range {
            min: 1,
            max: Some(2),
        },
        &[StdType::Int, StdType::Int],
        StdType::String,
    ),
    std_function(
        "write_stdin",
        StdFunctionKind::ProcessWriteStdin,
        StdArity::Exact(2),
        &[StdType::Int, StdType::String],
        StdType::Void,
    ),
    std_function(
        "close_stdin",
        StdFunctionKind::ProcessCloseStdin,
        StdArity::Exact(1),
        &[StdType::Int],
        StdType::Void,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.process",
    "Run external commands and manage subprocesses.",
    PROCESS_FUNCTIONS,
);
