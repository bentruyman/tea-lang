/// Intrinsic function implementations organized by category.
/// Each module contains helper functions that implement the actual intrinsic logic.
pub mod assert;
pub mod env;
pub mod fs;
pub mod path;
pub mod string;
pub mod util;

use anyhow::Result;

use super::value::Value;
use super::vm::Vm;

/// Type alias for intrinsic implementation functions
pub type IntrinsicImpl = fn(&mut Vm, Vec<Value>) -> Result<Value>;
