use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const FS_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "read_file",
        StdFunctionKind::FsReadText,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::String,
    ),
    std_function(
        "write_file",
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
        "remove",
        StdFunctionKind::FsRemove,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "read_dir",
        StdFunctionKind::FsListDir,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::List,
    ),
    std_function(
        "rename",
        StdFunctionKind::FsRename,
        StdArity::Exact(2),
        &[StdType::String, StdType::String],
        StdType::Void,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.fs",
    "Filesystem helpers for reading, writing, and inspecting paths.",
    FS_FUNCTIONS,
);
