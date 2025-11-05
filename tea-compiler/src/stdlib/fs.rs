use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const FS_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "read_text",
        StdFunctionKind::FsReadText,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "write_text",
        StdFunctionKind::FsWriteText,
        StdArity::Exact(2),
        &[StdType::String, StdType::String],
        StdType::Void,
    ),
    std_function(
        "create_dir",
        StdFunctionKind::FsCreateDir,
        StdArity::Range {
            min: 1,
            max: Some(2),
        },
        &[StdType::String, StdType::Bool],
        StdType::Void,
    ),
    std_function(
        "ensure_dir",
        StdFunctionKind::FsEnsureDir,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "remove",
        StdFunctionKind::FsRemove,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "exists",
        StdFunctionKind::FsExists,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Bool,
    ),
    std_function(
        "list_dir",
        StdFunctionKind::FsListDir,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::List,
    ),
    std_function(
        "walk",
        StdFunctionKind::FsWalk,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::List,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.fs",
    "Filesystem helpers for reading, writing, and inspecting paths.",
    FS_FUNCTIONS,
);
