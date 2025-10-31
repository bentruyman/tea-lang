use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const UTIL_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "to_string",
        StdFunctionKind::UtilToString,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::String,
    ),
    std_function(
        "clamp_int",
        StdFunctionKind::UtilClampInt,
        StdArity::Exact(3),
        &[StdType::Int, StdType::Int, StdType::Int],
        StdType::Int,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.util",
    "Utility predicates and helpers for runtime type inspection.",
    UTIL_FUNCTIONS,
);
