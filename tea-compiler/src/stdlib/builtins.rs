use super::{std_function, StdArity, StdFunction, StdFunctionKind, StdType};

/// Global built-in functions available without any `use` statement
pub const BUILTINS: &[StdFunction] = &[
    std_function(
        "print",
        StdFunctionKind::Print,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::Void,
    ),
    std_function(
        "println",
        StdFunctionKind::Println,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::Void,
    ),
    std_function(
        "to_string",
        StdFunctionKind::ToString,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::String,
    ),
    std_function(
        "type_of",
        StdFunctionKind::TypeOf,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::String,
    ),
    std_function(
        "len",
        StdFunctionKind::Length,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::Int,
    ),
    std_function(
        "exit",
        StdFunctionKind::Exit,
        StdArity::Exact(1),
        &[StdType::Int],
        StdType::Void,
    ),
    std_function(
        "delete",
        StdFunctionKind::Delete,
        StdArity::Exact(2),
        &[StdType::Dict, StdType::String],
        StdType::Dict,
    ),
    std_function(
        "clear",
        StdFunctionKind::Clear,
        StdArity::Exact(1),
        &[StdType::Dict],
        StdType::Dict,
    ),
    std_function(
        "max",
        StdFunctionKind::Max,
        StdArity::Exact(2),
        &[StdType::Any, StdType::Any],
        StdType::Any,
    ),
    std_function(
        "min",
        StdFunctionKind::Min,
        StdArity::Exact(2),
        &[StdType::Any, StdType::Any],
        StdType::Any,
    ),
    std_function(
        "append",
        StdFunctionKind::Append,
        StdArity::Exact(2),
        &[StdType::List, StdType::Any],
        StdType::List,
    ),
];
