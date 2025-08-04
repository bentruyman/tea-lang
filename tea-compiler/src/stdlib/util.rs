use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const UTIL_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "len",
        kind: StdFunctionKind::UtilLen,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Int,
    },
    StdFunction {
        name: "to_string",
        kind: StdFunctionKind::UtilToString,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::String,
    },
    StdFunction {
        name: "clamp_int",
        kind: StdFunctionKind::UtilClampInt,
        arity: StdArity::Exact(3),
        params: &[StdType::Int, StdType::Int, StdType::Int],
        return_type: StdType::Int,
    },
    StdFunction {
        name: "is_nil",
        kind: StdFunctionKind::UtilIsNil,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "is_bool",
        kind: StdFunctionKind::UtilIsBool,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "is_int",
        kind: StdFunctionKind::UtilIsInt,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "is_float",
        kind: StdFunctionKind::UtilIsFloat,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "is_string",
        kind: StdFunctionKind::UtilIsString,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "is_list",
        kind: StdFunctionKind::UtilIsList,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
    StdFunction {
        name: "is_struct",
        kind: StdFunctionKind::UtilIsStruct,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::Bool,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.util",
    functions: UTIL_FUNCTIONS,
};
