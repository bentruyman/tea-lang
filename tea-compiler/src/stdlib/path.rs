use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const PATH_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "join",
        kind: StdFunctionKind::PathJoin,
        arity: StdArity::Exact(1),
        params: &[StdType::List],
        return_type: StdType::String,
    },
    StdFunction {
        name: "components",
        kind: StdFunctionKind::PathComponents,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::List,
    },
    StdFunction {
        name: "dirname",
        kind: StdFunctionKind::PathDirname,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "basename",
        kind: StdFunctionKind::PathBasename,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "extension",
        kind: StdFunctionKind::PathExtension,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "set_extension",
        kind: StdFunctionKind::PathSetExtension,
        arity: StdArity::Exact(2),
        params: &[StdType::String, StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "strip_extension",
        kind: StdFunctionKind::PathStripExtension,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "normalize",
        kind: StdFunctionKind::PathNormalize,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "absolute",
        kind: StdFunctionKind::PathAbsolute,
        arity: StdArity::Range {
            min: 1,
            max: Some(2),
        },
        params: &[StdType::String, StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "relative",
        kind: StdFunctionKind::PathRelative,
        arity: StdArity::Exact(2),
        params: &[StdType::String, StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "is_absolute",
        kind: StdFunctionKind::PathIsAbsolute,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "separator",
        kind: StdFunctionKind::PathSeparator,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::String,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.path",
    functions: PATH_FUNCTIONS,
};
