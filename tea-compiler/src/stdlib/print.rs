use super::{debug::PRINT_FUNCTIONS, std_module, StdModule};

pub const MODULE: StdModule = std_module!(
    "std.print",
    "Printing utilities (deprecated alias for std.debug).",
    PRINT_FUNCTIONS,
);
