use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const ENV_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "get",
        StdFunctionKind::EnvGet,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "has",
        StdFunctionKind::EnvHas,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Bool,
    ),
    std_function(
        "set",
        StdFunctionKind::EnvSet,
        StdArity::Exact(2),
        &[StdType::String, StdType::String],
        StdType::Void,
    ),
    std_function(
        "unset",
        StdFunctionKind::EnvUnset,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "vars",
        StdFunctionKind::EnvVars,
        StdArity::Exact(0),
        &[],
        StdType::Dict,
    ),
    std_function(
        "cwd",
        StdFunctionKind::EnvCwd,
        StdArity::Exact(0),
        &[],
        StdType::String,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.env",
    "Environment variable and filesystem context helpers.",
    ENV_FUNCTIONS,
);
