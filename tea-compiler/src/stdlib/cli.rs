use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const CLI_FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "capture",
        kind: StdFunctionKind::CliCapture,
        arity: StdArity::Exact(1),
        params: &[StdType::List],
        return_type: StdType::Struct,
    },
    StdFunction {
        name: "args",
        kind: StdFunctionKind::CliArgs,
        arity: StdArity::Exact(0),
        params: &[],
        return_type: StdType::List,
    },
    StdFunction {
        name: "parse",
        kind: StdFunctionKind::CliParse,
        arity: StdArity::Range {
            min: 1,
            max: Some(2),
        },
        params: &[StdType::Dict, StdType::List],
        return_type: StdType::Struct,
    },
];

pub const MODULE: StdModule = StdModule {
    path: "support.cli",
    functions: CLI_FUNCTIONS,
};
