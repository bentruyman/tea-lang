use anyhow::{bail, Result};

use crate::runtime::value::Value;
use crate::runtime::vm::{Vm, VmError};

pub fn fail(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "fail expected 1 argument but got {}",
            args.len()
        )));
    }
    let message = match &args[0] {
        Value::String(text) => text.clone(),
        _ => bail!(VmError::Runtime(
            "fail expects a String message".to_string()
        )),
    };
    Err(VmError::Runtime(message).into())
}

pub fn snapshot(vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if !(2..=3).contains(&args.len()) {
        bail!(VmError::Runtime(format!(
            "assert_snapshot expected 2 or 3 arguments but got {}",
            args.len()
        )));
    }
    let name = vm.expect_string(&args[0], "assert_snapshot name must be a String")?;
    let actual = vm.expect_string(&args[1], "assert_snapshot value must be a String")?;
    let label = if args.len() == 3 {
        Some(vm.expect_string(&args[2], "snapshot label must be a String")?)
    } else {
        None
    };
    vm.handle_snapshot_assertion(&name, &actual, label.as_deref())?;
    Ok(Value::Void)
}
