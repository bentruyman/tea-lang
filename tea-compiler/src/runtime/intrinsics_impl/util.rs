use anyhow::Result;

use crate::runtime::value::Value;
use crate::runtime::vm::Vm;

pub fn to_string(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    // Note: This is the same as string::to_string but kept separate for clarity
    // as it's used internally for interpolation
    Ok(Value::String(args[0].to_string()))
}
