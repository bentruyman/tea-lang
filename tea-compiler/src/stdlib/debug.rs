use super::{std_function, std_module, StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

pub(super) const PRINT_FUNCTIONS: &[StdFunction] = &[std_function(
    "print",
    StdFunctionKind::Print,
    StdArity::Exact(1),
    &[StdType::Any],
    StdType::Void,
)];

pub const MODULE: StdModule = std_module!(
    "std.debug",
    "Debug utilities such as printing.",
    PRINT_FUNCTIONS,
);
