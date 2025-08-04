use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const ENV_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "get",
        kind: StdFunctionKind::EnvGet,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "get_or",
        kind: StdFunctionKind::EnvGetOr,
        arity: StdArity::Exact(2),
        params: &[StdType::String, StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "has",
        kind: StdFunctionKind::EnvHas,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "require",
        kind: StdFunctionKind::EnvRequire,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::String,
    },
    StdFunction {
        name: "set",
        kind: StdFunctionKind::EnvSet,
        arity: StdArity::Exact(2),
        params: &[StdType::String, StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "unset",
        kind: StdFunctionKind::EnvUnset,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "vars",
        kind: StdFunctionKind::EnvVars,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::Dict,
    },
    StdFunction {
        name: "cwd",
        kind: StdFunctionKind::EnvCwd,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::String,
    },
    StdFunction {
        name: "set_cwd",
        kind: StdFunctionKind::EnvSetCwd,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "temp_dir",
        kind: StdFunctionKind::EnvTempDir,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::String,
    },
    StdFunction {
        name: "home_dir",
        kind: StdFunctionKind::EnvHomeDir,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::String,
    },
    StdFunction {
        name: "config_dir",
        kind: StdFunctionKind::EnvConfigDir,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::String,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.env",
    functions: ENV_FUNCTIONS,
};
