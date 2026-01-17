use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const ARGS_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "all",
        StdFunctionKind::ArgsAll,
        StdArity::Exact(0),
        &[],
        StdType::List,
    ),
    std_function(
        "program",
        StdFunctionKind::ArgsProgram,
        StdArity::Exact(0),
        &[],
        StdType::String,
    ),
    // Note: has, get, and positional are implemented in pure Tea
    // and don't need compiler-side registration
];

pub const MODULE: StdModule = std_module!(
    "std.args",
    "Command-line argument parsing utilities.",
    ARGS_FUNCTIONS,
);
