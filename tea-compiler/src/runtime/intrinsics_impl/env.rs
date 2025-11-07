use anyhow::{bail, Result};
use std::rc::Rc;

use crate::runtime::value::Value;
use crate::runtime::vm::{Vm, VmError};

pub fn get(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "env_get expected 1 argument but got {}",
            args.len()
        )));
    }
    let key = match &args[0] {
        Value::String(k) => k.as_str(),
        _ => bail!(VmError::Runtime("env_get expects a String key".to_string())),
    };
    let value = tea_intrinsics::env::get(key);
    Ok(Value::String(value))
}

pub fn has(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "env_has expected 1 argument but got {}",
            args.len()
        )));
    }
    let key = match &args[0] {
        Value::String(k) => k.as_str(),
        _ => bail!(VmError::Runtime("env_has expects a String key".to_string())),
    };
    Ok(Value::Bool(tea_intrinsics::env::has(key)))
}

pub fn set(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "env_set expected 2 arguments but got {}",
            args.len()
        )));
    }
    let (key, value) = match (&args[0], &args[1]) {
        (Value::String(k), Value::String(v)) => (k.as_str(), v.as_str()),
        _ => bail!(VmError::Runtime(
            "env_set expects two String arguments".to_string()
        )),
    };
    tea_intrinsics::env::set(key, value);
    Ok(Value::Nil)
}

pub fn unset(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "env_unset expected 1 argument but got {}",
            args.len()
        )));
    }
    let key = match &args[0] {
        Value::String(k) => k.as_str(),
        _ => bail!(VmError::Runtime(
            "env_unset expects a String key".to_string()
        )),
    };
    tea_intrinsics::env::unset(key);
    Ok(Value::Nil)
}

pub fn vars(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if !args.is_empty() {
        bail!(VmError::Runtime(format!(
            "env_vars expected 0 arguments but got {}",
            args.len()
        )));
    }
    let env_vars = tea_intrinsics::env::vars();
    let mut map = std::collections::HashMap::new();
    for (key, value) in env_vars {
        map.insert(key, Value::String(value));
    }
    Ok(Value::Dict(Rc::new(map)))
}

pub fn cwd(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if !args.is_empty() {
        bail!(VmError::Runtime(format!(
            "env_cwd expected 0 arguments but got {}",
            args.len()
        )));
    }
    let current_dir = tea_intrinsics::env::cwd().map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::String(current_dir))
}
