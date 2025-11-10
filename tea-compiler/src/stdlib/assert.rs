use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const ASSERT_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "ok",
        StdFunctionKind::Assert,
        StdArity::Exact(1),
        &[StdType::Any],
        StdType::Void,
    ),
    std_function(
        "eq",
        StdFunctionKind::AssertEq,
        StdArity::Exact(2),
        &[StdType::Any, StdType::Any],
        StdType::Void,
    ),
    std_function(
        "ne",
        StdFunctionKind::AssertNe,
        StdArity::Exact(2),
        &[StdType::Any, StdType::Any],
        StdType::Void,
    ),
    std_function(
        "snapshot",
        StdFunctionKind::AssertSnapshot,
        StdArity::Range {
            min: 2,
            max: Some(3),
        },
        &[StdType::String, StdType::String, StdType::String],
        StdType::Void,
    ),
];

pub const MODULE: StdModule = std_module!(
    "std.assert",
    "Assertion helpers for tests and runtime checks.",
    ASSERT_FUNCTIONS,
);
