use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const JSON_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "encode",
        StdFunctionKind::JsonEncode,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::String,
    ),
    std_function(
        "decode",
        StdFunctionKind::JsonDecode,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Any,
    ),
];

pub const MODULE: StdModule =
    std_module!("std.json", "JSON encode/decode helpers.", JSON_FUNCTIONS,);
