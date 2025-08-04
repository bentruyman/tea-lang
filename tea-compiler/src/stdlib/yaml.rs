use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const YAML_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "encode",
        kind: StdFunctionKind::YamlEncode,
        arity: StdArity::Exact(1),
        params: &[StdType::Any],
        return_type: StdType::String,
    },
    StdFunction {
        name: "decode",
        kind: StdFunctionKind::YamlDecode,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Any,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.yaml",
    functions: YAML_FUNCTIONS,
};
