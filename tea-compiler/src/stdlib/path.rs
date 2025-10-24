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
        "components",
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
    std_function(
        "set_extension",
        StdFunctionKind::PathSetExtension,
        StdArity::Exact(2),
        &[StdType::String, StdType::String],
        StdType::String,
    ),
    std_function(
        "strip_extension",
        StdFunctionKind::PathStripExtension,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "normalize",
        StdFunctionKind::PathNormalize,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "absolute",
        StdFunctionKind::PathAbsolute,
        StdArity::Range {
            min: 1,
            max: Some(2),
        },
        &[StdType::String, StdType::String],
        StdType::String,
    ),
    std_function(
        "relative",
        StdFunctionKind::PathRelative,
        StdArity::Exact(2),
        &[StdType::String, StdType::String],
        StdType::String,
    ),
    std_function(
        "is_absolute",
        StdFunctionKind::PathIsAbsolute,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Bool,
    ),
    std_function(
        "separator",
        StdFunctionKind::PathSeparator,
        StdArity::Exact(0),
        &[],
        StdType::String,
    ),
];

pub const MODULE: StdModule =
    std_module!("std.path", "Path manipulation helpers.", PATH_FUNCTIONS,);
