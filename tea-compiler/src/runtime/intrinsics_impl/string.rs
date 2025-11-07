use anyhow::{bail, Result};
use std::rc::Rc;

use crate::runtime::value::Value;
use crate::runtime::vm::{Vm, VmError};

pub fn to_string(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 1 {
        bail!(VmError::Runtime(format!(
            "to_string expected 1 argument but got {}",
            args.len()
        )));
    }
    Ok(Value::String(args[0].to_string()))
}

pub fn index_of(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "string_index_of expected 2 arguments but got {}",
            args.len()
        )));
    }
    let (haystack, needle) = match (&args[0], &args[1]) {
        (Value::String(h), Value::String(n)) => (h.as_str(), n.as_str()),
        _ => bail!(VmError::Runtime(
            "string_index_of expects two String arguments".to_string()
        )),
    };
    let index = tea_intrinsics::string::index_of(haystack, needle);
    Ok(Value::Int(index))
}

pub fn split(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "string_split expected 2 arguments but got {}",
            args.len()
        )));
    }
    let (text, delimiter) = match (&args[0], &args[1]) {
        (Value::String(t), Value::String(d)) => (t.as_str(), d.as_str()),
        _ => bail!(VmError::Runtime(
            "string_split expects two String arguments".to_string()
        )),
    };
    let parts = tea_intrinsics::string::split(text, delimiter);
    let values: Vec<Value> = parts.into_iter().map(Value::String).collect();
    Ok(Value::List(Rc::new(values)))
}

pub fn contains(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 2 {
        bail!(VmError::Runtime(format!(
            "string_contains expected 2 arguments but got {}",
            args.len()
        )));
    }
    let (haystack, needle) = match (&args[0], &args[1]) {
        (Value::String(h), Value::String(n)) => (h.as_str(), n.as_str()),
        _ => bail!(VmError::Runtime(
            "string_contains expects two String arguments".to_string()
        )),
    };
    Ok(Value::Bool(tea_intrinsics::string::contains(
        haystack, needle,
    )))
}

pub fn replace(_vm: &mut Vm, args: Vec<Value>) -> Result<Value> {
    if args.len() != 3 {
        bail!(VmError::Runtime(format!(
            "string_replace expected 3 arguments but got {}",
            args.len()
        )));
    }
    let (text, search, replacement) = match (&args[0], &args[1], &args[2]) {
        (Value::String(t), Value::String(s), Value::String(r)) => {
            (t.as_str(), s.as_str(), r.as_str())
        }
        _ => bail!(VmError::Runtime(
            "string_replace expects three String arguments".to_string()
        )),
    };
    let result = tea_intrinsics::string::replace(text, search, replacement);
    Ok(Value::String(result))
}
