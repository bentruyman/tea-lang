use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const ASSERT_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "assert",
        StdFunctionKind::Assert,
        StdArity::Range {
            min: 1,
            max: Some(2),
        },
        &[StdType::Bool, StdType::String],
        StdType::Void,
    ),
    std_function(
        "assert_eq",
        StdFunctionKind::AssertEq,
        StdArity::Exact(2),
        &[StdType::Any, StdType::Any],
        StdType::Void,
    ),
    std_function(
        "assert_ne",
        StdFunctionKind::AssertNe,
        StdArity::Exact(2),
        &[StdType::Any, StdType::Any],
        StdType::Void,
    ),
    std_function(
        "fail",
        StdFunctionKind::AssertFail,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
    std_function(
        "assert_snapshot",
        StdFunctionKind::AssertSnapshot,
        StdArity::Range {
            min: 2,
            max: Some(3),
        },
        &[StdType::String, StdType::String, StdType::String],
        StdType::Void,
    ),
    std_function(
        "assert_empty",
        StdFunctionKind::AssertEmpty,
        StdArity::Exact(1),
        &[StdType::String],
        StdType::Void,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.assert",
    "Assertion helpers for tests and runtime checks.",
    ASSERT_FUNCTIONS,
);
