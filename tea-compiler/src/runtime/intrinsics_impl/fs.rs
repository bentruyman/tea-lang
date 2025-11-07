use anyhow::{bail, Result};
use std::rc::Rc;

use crate::runtime::value::Value;
use crate::runtime::vm::{Vm, VmError};

fn vm_strings_to_list(strings: Vec<String>) -> Value {
    let values: Vec<Value> = strings.into_iter().map(Value::String).collect();
    Value::List(Rc::new(values))
}

pub fn read_text(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "read_text expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "read_text expects a String path".to_string()
        )),
    };
    let contents =
        tea_intrinsics::fs::read_text(path).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::String(contents))
}

pub fn write_text(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "write_text expected 2 arguments but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "write_text expects the path to be a String".to_string()
        )),
    };
    let contents = match &args[1] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "write_text expects the contents to be a String".to_string()
        )),
    };
    tea_intrinsics::fs::write_text(path, contents).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::Void)
}

pub fn create_dir(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "create_dir expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "create_dir expects the path to be a String".to_string()
        )),
    };
    tea_intrinsics::fs::create_dir(path).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::Void)
}

pub fn ensure_dir(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "ensure_dir expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "ensure_dir expects the path to be a String".to_string()
        )),
    };
    tea_intrinsics::fs::ensure_dir(path).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::Void)
}

pub fn remove(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "remove expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "remove expects the path to be a String".to_string()
        )),
    };
    tea_intrinsics::fs::remove(path).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::Void)
}

pub fn exists(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "exists expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "exists expects the path to be a String".to_string()
        )),
    };
    Ok(Value::Bool(tea_intrinsics::fs::exists(path)))
}

pub fn list_dir(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "list_dir expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "list_dir expects the path to be a String".to_string()
        )),
    };
    let entries =
        tea_intrinsics::fs::list_dir(path).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(vm_strings_to_list(entries))
}

pub fn walk(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "walk expected 1 argument but got {}",
            args.len()
        )));
    }
    let path = match &args[0] {
        Value::String(text) => text.as_str(),
        _ => bail!(VmError::Runtime(
            "walk expects the path to be a String".to_string()
        )),
    };
    let entries = tea_intrinsics::fs::walk(path).map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(vm_strings_to_list(entries))
}
