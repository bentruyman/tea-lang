use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const ASSERT_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "assert",
        kind: StdFunctionKind::Assert,
        arity: StdArity::Range {
            min: 1,
            max: Some(2),
        },
        params: &[StdType::Bool, StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "assert_eq",
        kind: StdFunctionKind::AssertEq,
        arity: StdArity::Exact(2),
        params: &[StdType::Any, StdType::Any],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "assert_ne",
        kind: StdFunctionKind::AssertNe,
        arity: StdArity::Exact(2),
        params: &[StdType::Any, StdType::Any],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "fail",
        kind: StdFunctionKind::AssertFail,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "assert_snapshot",
        kind: StdFunctionKind::AssertSnapshot,
        arity: StdArity::Range {
            min: 2,
            max: Some(3),
        },
        params: &[StdType::String, StdType::String, StdType::String],
        return_type: StdType::Nil,
    },
    StdFunction {
        name: "assert_empty",
        kind: StdFunctionKind::AssertEmpty,
        arity: StdArity::Exact(1),
        params: &[StdType::String],
        return_type: StdType::Nil,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "std.assert",
    functions: ASSERT_FUNCTIONS,
};
