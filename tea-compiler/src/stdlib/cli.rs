use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

const CLI_FUNCTIONS: &[StdFunction] = &[
    std_function(
        "capture",
        StdFunctionKind::CliCapture,
        StdArity::Exact(1),
        &[StdType::List],
        StdType::Struct,
    ),
    std_function(
        "args",
        StdFunctionKind::CliArgs,
        StdArity::Exact(0),
        &[],
        StdType::List,
    ),
    std_function(
        "parse",
        StdFunctionKind::CliParse,
        StdArity::Range {
            min: 1,
            max: Some(2),
        },
        &[StdType::Dict, StdType::List],
        StdType::Struct,
    ),
];

pub const MODULE: StdModule = std_module!(
    "support.cli",
    "Support routines for building command-line interfaces.",
    CLI_FUNCTIONS,
);
