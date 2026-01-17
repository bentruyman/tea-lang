use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const REGEX_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "compile",
        StdFunctionKind::RegexCompile,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Int,
    ),
    std_function(
        "is_match",
        StdFunctionKind::RegexIsMatch,
        StdArity::Exact(2),
        &[StdType::Int, StdType::String],
        StdType::Bool,
    ),
    std_function(
        "find_all",
        StdFunctionKind::RegexFindAll,
        StdArity::Exact(2),
        &[StdType::Int, StdType::String],
        StdType::List,
    ),
    std_function(
        "captures",
        StdFunctionKind::RegexCaptures,
        StdArity::Exact(2),
        &[StdType::Int, StdType::String],
        StdType::List,
    ),
    std_function(
        "replace",
        StdFunctionKind::RegexReplace,
        StdArity::Exact(3),
        &[StdType::Int, StdType::String, StdType::String],
        StdType::String,
    ),
    std_function(
        "replace_all",
        StdFunctionKind::RegexReplaceAll,
        StdArity::Exact(3),
        &[StdType::Int, StdType::String, StdType::String],
        StdType::String,
    ),
    std_function(
        "split",
        StdFunctionKind::RegexSplit,
        StdArity::Exact(2),
        &[StdType::Int, StdType::String],
        StdType::List,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.regex",
    "Regular expression pattern matching and text manipulation.",
    REGEX_FUNCTIONS,
);
