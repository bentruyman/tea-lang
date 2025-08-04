use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const JSON_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "encode",
        kind: StdFunctionKind::JsonEncode,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::String,
    },
    StdFunction {
        name: "decode",
        kind: StdFunctionKind::JsonDecode,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Any,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.json",
    functions: JSON_FUNCTIONS,
};
