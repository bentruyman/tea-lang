use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const PATH_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "join",
        StdFunctionKind::PathJoin,
        StdArity::Exact(1),
        &[StdType::List],
        StdType::String,
    ),
    std_function(
        "split",
        StdFunctionKind::PathComponents,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::List,
    ),
    std_function(
        "dirname",
        StdFunctionKind::PathDirname,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "basename",
        StdFunctionKind::PathBasename,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "extension",
        StdFunctionKind::PathExtension,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
];

pub const MODULE: StdModule =
    std_module!("std.path", "Path manipulation helpers.", PATH_FUNCTIONS,);
