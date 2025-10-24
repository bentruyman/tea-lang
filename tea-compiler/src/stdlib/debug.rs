use super::{StdArity, StdFunction, StdFunctionKind, StdModule, StdType};

pub(super) const PRINT_FUNCTIONS: &[StdFunction] = &[StdFunction {
    name: "print",
    kind: StdFunctionKind::Print,
    arity: StdArity::Exact(1),
    params: &[StdType::Any],
    return_type: StdType::Nil,
}];

pub const MODULE: StdModule = StdModule {
    path: "std.debug",
    functions: PRINT_FUNCTIONS,
};
