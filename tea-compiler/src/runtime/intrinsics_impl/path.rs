use anyhow::{bail, Result};
use std::rc::Rc;

use crate::runtime::value::Value;
use crate::runtime::vm::{Vm, VmError};

fn vm_strings_to_list(strings: Vec<String>) -> Value {
    let values: Vec<Value> = strings.into_iter().map(Value::String).collect();
    Value::List(Rc::new(values))
}

pub fn separator(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if !args.is_empty() {
        bail!(VmError::Runtime(format!(
            "separator expected 0 arguments but got {}",
            args.len()
        )));
    }
    Ok(Value::String(tea_intrinsics::path::separator()))
}

pub fn join(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "join expected 1 argument but got {}",
            args.len()
        )));
    }
    let parts = match &args[0] {
        Value::List(items) => items,
        _ => bail!(VmError::Runtime(
            "join expects a List of path parts".to_string()
        )),
    };
    let mut string_parts = Vec::new();
    for part in parts.iter() {
        match part {
            Value::String(s) => string_parts.push(s.clone()),
            _ => bail!(VmError::Runtime(
                "join expects all path parts to be Strings".to_string()
            )),
        }
    }
    let result = tea_intrinsics::path::join(&string_parts);
    Ok(Value::String(result))
}

pub fn components(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "components expected 1 argument but got {}",
            args.len()
        )));
    }
    let path_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "components expects a String path".to_string()
        )),
    };
    let parts = tea_intrinsics::path::components(path_str);
    Ok(vm_strings_to_list(parts))
}

pub fn dirname(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "dirname expected 1 argument but got {}",
            args.len()
        )));
    }
    let path_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "dirname expects a String path".to_string()
        )),
    };
    let parent = tea_intrinsics::path::dirname(path_str);
    Ok(Value::String(parent))
}

pub fn basename(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "basename expected 1 argument but got {}",
            args.len()
        )));
    }
    let path_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "basename expects a String path".to_string()
        )),
    };
    let name = tea_intrinsics::path::basename(path_str);
    Ok(Value::String(name))
}

pub fn extension(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "extension expected 1 argument but got {}",
            args.len()
        )));
    }
    let path_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "extension expects a String path".to_string()
        )),
    };
    let ext = tea_intrinsics::path::extension(path_str);
    Ok(Value::String(ext))
}

pub fn normalize(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "normalize expected 1 argument but got {}",
            args.len()
        )));
    }
    let path_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "normalize expects a String path".to_string()
        )),
    };
    let normalized = tea_intrinsics::path::normalize(path_str);
    Ok(Value::String(normalized))
}

pub fn absolute(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if !(1..=2).contains(&args.len()) {
        bail!(VmError::Runtime(format!(
            "absolute expected 1 or 2 arguments but got {}",
            args.len()
        )));
    }
    let path_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => bail!(VmError::Runtime(
            "absolute expects a String path".to_string()
        )),
    };
    let base_str = if args.len() == 2 {
        Some(match &args[1] {
            Value::String(s) => s.as_str(),
            _ => bail!(VmError::Runtime(
                "absolute expects base to be a String".to_string()
            )),
        })
    } else {
        None
    };

    let result = tea_intrinsics::path::absolute(path_str, base_str)
        .map_err(|e| VmError::Runtime(e.to_string()))?;
    Ok(Value::String(result))
}

pub fn relative(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "relative expected 2 arguments but got {}",
            args.len()
        )));
    }
    let (from, to) = match (&args[0], &args[1]) {
        (Value::String(f), Value::String(t)) => (f.as_str(), t.as_str()),
        _ => bail!(VmError::Runtime(
            "relative expects two String arguments".to_string()
        )),
    };
    let result = tea_intrinsics::path::relative(from, to);
    Ok(Value::String(result))
}
