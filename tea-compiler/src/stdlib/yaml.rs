use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const YAML_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "encode",
        StdFunctionKind::YamlEncode,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::String,
    ),
    std_function(
        "decode",
        StdFunctionKind::YamlDecode,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Any,
    ),
];

pub const MODULE: StdModule =
    std_module!("std.yaml", "YAML encode/decode helpers.", YAML_FUNCTIONS,);
