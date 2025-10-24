mod cli;

use crate::cli::{CliParseOutcome, CliScopeOutcome, RuntimeValue};
use anyhow::{anyhow, Context, Result};
use dirs_next::{config_dir, home_dir};
use glob::glob;
use path_clean::PathClean;
use pathdiff::diff_paths;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::env;
use std::ffi::{c_void, CStr};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::os::raw::{c_char, c_double, c_int, c_longlong};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use tea_support::{cli_error, env_error, fs_error, io_error, process_error};
use tempfile::NamedTempFile;
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TeaString {
    pub len: c_longlong,
    pub data: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TeaList {
    pub len: c_longlong,
    pub capacity: c_longlong,
    pub items: *mut TeaValue,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TeaStructTemplate {
    pub name: *const c_char,
    pub field_count: c_longlong,
    pub field_names: *const *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TeaStructInstance {
    pub template: *const TeaStructTemplate,
    pub fields: *mut TeaValue,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TeaClosure {
    pub function: *const c_void,
    pub captures: *mut TeaValue,
    pub capture_count: c_longlong,
}

pub struct TeaDict {
    entries: HashMap<String, TeaValue>,
}

struct FsHandle {
    reader: BufReader<File>,
}

static FS_HANDLES: OnceLock<Mutex<HashMap<i64, FsHandle>>> = OnceLock::new();
static NEXT_FS_HANDLE: AtomicI64 = AtomicI64::new(1);

fn fs_handles() -> &'static Mutex<HashMap<i64, FsHandle>> {
    FS_HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

struct ProcessHandleEntry {
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
    stderr: Option<BufReader<ChildStderr>>,
    stdin: Option<ChildStdin>,
    command: String,
}

static PROCESS_HANDLES: OnceLock<Mutex<HashMap<i64, ProcessHandleEntry>>> = OnceLock::new();
static NEXT_PROCESS_HANDLE: AtomicI64 = AtomicI64::new(1);

fn process_handles() -> &'static Mutex<HashMap<i64, ProcessHandleEntry>> {
    PROCESS_HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn alloc_tea_string(text: &str) -> *mut TeaString {
    let bytes = text.as_bytes();
    tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong)
}

fn dict_set_value(dict: *mut TeaDict, key: &str, value: TeaValue) {
    let key_ptr = alloc_tea_string(key);
    tea_dict_set(dict, key_ptr, value);
}

fn dict_set_bool(dict: *mut TeaDict, key: &str, value: bool) {
    let bool_value = tea_value_from_bool(if value { 1 } else { 0 });
    dict_set_value(dict, key, bool_value);
}

fn dict_set_int(dict: *mut TeaDict, key: &str, value: i64) {
    let int_value = tea_value_from_int(value as c_longlong);
    dict_set_value(dict, key, int_value);
}

fn dict_set_string(dict: *mut TeaDict, key: &str, value: &str) {
    let value_ptr = alloc_tea_string(value);
    let string_value = tea_value_from_string(value_ptr);
    dict_set_value(dict, key, string_value);
}

fn dict_set_optional_int(dict: *mut TeaDict, key: &str, value: Option<i64>) {
    match value {
        Some(number) => dict_set_int(dict, key, number),
        None => dict_set_value(dict, key, tea_value_nil()),
    }
}

fn strings_to_list(items: Vec<String>) -> *mut TeaList {
    let list = tea_alloc_list(items.len() as c_longlong);
    for (index, item) in items.into_iter().enumerate() {
        let string_ptr = alloc_tea_string(&item);
        tea_list_set(list, index as c_longlong, tea_value_from_string(string_ptr));
    }
    list
}

fn write_atomic_bytes(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(data)?;
    temp.flush()?;
    temp.persist(path).map(|_| ()).map_err(|error| error.error)
}

fn metadata_mode(metadata: &fs::Metadata) -> i64 {
    #[cfg(unix)]
    {
        metadata.permissions().mode() as i64
    }
    #[cfg(windows)]
    {
        metadata.file_attributes() as i64
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        0
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum TeaValueTag {
    Int,
    Float,
    Bool,
    String,
    List,
    Dict,
    Struct,
    Closure,
    Nil,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union TeaValuePayload {
    pub int_value: c_longlong,
    pub float_value: c_double,
    pub bool_value: c_int,
    pub string_value: *const TeaString,
    pub list_value: *const TeaList,
    pub dict_value: *const TeaDict,
    pub struct_value: *const TeaStructInstance,
    pub closure_value: *const TeaClosure,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TeaValue {
    pub tag: TeaValueTag,
    pub payload: TeaValuePayload,
}

#[no_mangle]
pub extern "C" fn tea_print_int(value: c_longlong) {
    println!("{value}");
}

#[no_mangle]
pub extern "C" fn tea_print_float(value: c_double) {
    println!("{value}");
}

#[no_mangle]
pub extern "C" fn tea_print_bool(value: c_int) {
    println!("{}", value != 0);
}

#[no_mangle]
pub extern "C" fn tea_print_string(value: *const TeaString) {
    unsafe {
        if value.is_null() {
            println!("(null)");
            return;
        }
        let string_ref = &*value;
        let bytes =
            std::slice::from_raw_parts(string_ref.data as *const u8, string_ref.len as usize);
        match std::str::from_utf8(bytes) {
            Ok(text) => println!("{text}"),
            Err(_) => println!("<invalid utf8>"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_print_list(list: *const TeaList) {
    unsafe {
        if list.is_null() {
            println!("[]");
            return;
        }
        let list_ref = &*list;
        print!("[");
        for i in 0..list_ref.len {
            if i > 0 {
                print!(", ");
            }
            let value = *list_ref.items.add(i as usize);
            print_value(value);
        }
        println!("]");
    }
}

fn print_value(value: TeaValue) {
    unsafe {
        match value.tag {
            TeaValueTag::Int => print!("{}", value.payload.int_value),
            TeaValueTag::Float => print!("{}", value.payload.float_value),
            TeaValueTag::Bool => print!("{}", value.payload.bool_value != 0),
            TeaValueTag::Nil => print!("nil"),
            TeaValueTag::String => tea_print_string(value.payload.string_value),
            TeaValueTag::List => tea_print_list(value.payload.list_value),
            TeaValueTag::Dict => tea_print_dict(value.payload.dict_value),
            TeaValueTag::Struct => tea_print_struct(value.payload.struct_value),
            TeaValueTag::Closure => tea_print_closure(value.payload.closure_value),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_print_dict(dict: *const TeaDict) {
    unsafe {
        if dict.is_null() {
            println!("{{}}");
            return;
        }
        let dict_ref = &*dict;
        print!("{{");
        let mut first = true;
        for (key, value) in dict_ref.entries.iter() {
            if !first {
                print!(", ");
            }
            first = false;
            print!("{key}: ");
            print_value(*value);
        }
        println!("}}");
    }
}

fn tea_cstr_to_rust(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

fn tea_string_to_rust(ptr: *const TeaString) -> Option<String> {
    unsafe {
        if ptr.is_null() {
            return None;
        }
        let string_ref = &*ptr;
        if string_ref.data.is_null() || string_ref.len < 0 {
            return None;
        }
        let bytes =
            std::slice::from_raw_parts(string_ref.data as *const u8, string_ref.len as usize);
        Some(String::from_utf8_lossy(bytes).to_string())
    }
}

fn expect_string(ptr: *const TeaString, context: &str) -> String {
    tea_string_to_rust(ptr).unwrap_or_else(|| panic!("{context}"))
}

fn expect_path(path: *const TeaString) -> String {
    expect_string(path, "fs path must be a valid UTF-8 string")
}

fn expect_string_list_from_list(list: *const TeaList, context: &str) -> Vec<String> {
    if list.is_null() {
        panic!("{context}");
    }
    let list_ref = unsafe { &*list };
    let mut values = Vec::with_capacity(list_ref.len as usize);
    for index in 0..list_ref.len {
        let value = tea_list_get(list, index);
        let string_ptr = tea_value_as_string(value);
        values.push(expect_string(
            string_ptr,
            "expected list elements to be valid UTF-8 strings",
        ));
    }
    values
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn compute_dirname(input: &str) -> String {
    let path = Path::new(input);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => path_to_string(parent),
        Some(_) => {
            if path.has_root() {
                path_to_string(path)
            } else {
                ".".to_string()
            }
        }
        None => {
            if path.has_root() {
                path_to_string(path)
            } else {
                ".".to_string()
            }
        }
    }
}

fn compute_basename(input: &str) -> String {
    let path = Path::new(input);
    let mut last = None;
    for component in path.components() {
        last = Some(component);
    }
    match last {
        Some(Component::Normal(part)) => part.to_string_lossy().into_owned(),
        Some(Component::CurDir) => ".".to_string(),
        Some(Component::ParentDir) => "..".to_string(),
        Some(Component::RootDir) => std::path::MAIN_SEPARATOR.to_string(),
        Some(Component::Prefix(prefix)) => prefix.as_os_str().to_string_lossy().into_owned(),
        None => input.to_string(),
    }
}

fn compute_absolute(target: &str, base: Option<String>) -> Result<String> {
    let target_path = PathBuf::from(target);
    if target_path.is_absolute() {
        return Ok(path_to_string(target_path.clean().as_path()));
    }

    let mut base_path = if let Some(base) = base {
        PathBuf::from(base)
    } else {
        env::current_dir().context("failed to resolve current directory")?
    };

    if !base_path.is_absolute() {
        let cwd = env::current_dir().context("failed to resolve current directory")?;
        base_path = cwd.join(base_path);
    }

    let combined = base_path.join(target_path);
    Ok(path_to_string(combined.clean().as_path()))
}

fn compute_relative(target: &str, base: &str) -> Result<String> {
    let target_path = Path::new(target).clean();
    let base_path = Path::new(base).clean();
    match diff_paths(&target_path, &base_path) {
        Some(diff) => {
            if diff.as_os_str().is_empty() {
                Ok(".".to_string())
            } else {
                Ok(path_to_string(diff.as_path()))
            }
        }
        None => anyhow::bail!(
            "unable to compute relative path from '{}' to '{}'",
            base,
            target
        ),
    }
}

#[no_mangle]
pub extern "C" fn tea_path_join(parts: *const TeaList) -> *mut TeaString {
    let segments = expect_string_list_from_list(parts, "path.join expects a List[String]");
    let mut buf = PathBuf::new();
    for segment in segments {
        buf.push(segment);
    }
    let result = path_to_string(buf.as_path());
    alloc_tea_string(&result)
}

#[no_mangle]
pub extern "C" fn tea_path_components(path: *const TeaString) -> *mut TeaList {
    let input = expect_path(path);
    let mut parts = Vec::new();
    for component in Path::new(&input).components() {
        parts.push(component.as_os_str().to_string_lossy().into_owned());
    }
    strings_to_list(parts)
}

#[no_mangle]
pub extern "C" fn tea_path_dirname(path: *const TeaString) -> *mut TeaString {
    let input = expect_path(path);
    let dirname = compute_dirname(&input);
    alloc_tea_string(&dirname)
}

#[no_mangle]
pub extern "C" fn tea_path_basename(path: *const TeaString) -> *mut TeaString {
    let input = expect_path(path);
    let basename = compute_basename(&input);
    alloc_tea_string(&basename)
}

#[no_mangle]
pub extern "C" fn tea_path_extension(path: *const TeaString) -> *mut TeaString {
    let input = expect_path(path);
    let extension = Path::new(&input)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    alloc_tea_string(&extension)
}

#[no_mangle]
pub extern "C" fn tea_path_set_extension(
    path: *const TeaString,
    extension: *const TeaString,
) -> *mut TeaString {
    let input = expect_path(path);
    let ext = expect_string(
        extension,
        "path.set_extension expects the extension to be a valid UTF-8 string",
    );
    let mut buf = PathBuf::from(input);
    buf.set_extension(ext);
    let result = path_to_string(buf.as_path());
    alloc_tea_string(&result)
}

#[no_mangle]
pub extern "C" fn tea_path_strip_extension(path: *const TeaString) -> *mut TeaString {
    let input = expect_path(path);
    let mut buf = PathBuf::from(input);
    buf.set_extension("");
    let result = path_to_string(buf.as_path());
    alloc_tea_string(&result)
}

#[no_mangle]
pub extern "C" fn tea_path_normalize(path: *const TeaString) -> *mut TeaString {
    let input = expect_path(path);
    let normalized = Path::new(&input).clean();
    let result = path_to_string(normalized.as_path());
    alloc_tea_string(&result)
}

#[no_mangle]
pub extern "C" fn tea_path_absolute(
    path: *const TeaString,
    base: *const TeaString,
    has_base: c_int,
) -> *mut TeaString {
    let input = expect_path(path);
    let base_value = if has_base != 0 {
        Some(expect_path(base))
    } else {
        None
    };
    let resolved = compute_absolute(&input, base_value).unwrap_or_else(|error| panic!("{error}"));
    alloc_tea_string(&resolved)
}

#[no_mangle]
pub extern "C" fn tea_path_relative(
    target: *const TeaString,
    base: *const TeaString,
) -> *mut TeaString {
    let target_path = expect_path(target);
    let base_path = expect_path(base);
    let relative =
        compute_relative(&target_path, &base_path).unwrap_or_else(|error| panic!("{error}"));
    alloc_tea_string(&relative)
}

#[no_mangle]
pub extern "C" fn tea_path_is_absolute(path: *const TeaString) -> c_int {
    let input = expect_path(path);
    if Path::new(&input).is_absolute() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_path_separator() -> *mut TeaString {
    let sep = std::path::MAIN_SEPARATOR.to_string();
    alloc_tea_string(&sep)
}

#[no_mangle]
pub extern "C" fn tea_env_get(name: *const TeaString) -> *mut TeaString {
    let key = expect_string(name, "env.get expects the name to be a valid UTF-8 string");
    let value = env::var(&key).unwrap_or_default();
    alloc_tea_string(&value)
}

#[no_mangle]
pub extern "C" fn tea_env_get_or(
    name: *const TeaString,
    fallback: *const TeaString,
) -> *mut TeaString {
    let key = expect_string(
        name,
        "env.get_or expects the name to be a valid UTF-8 string",
    );
    let default = expect_string(
        fallback,
        "env.get_or expects the fallback to be a valid UTF-8 string",
    );
    let value = env::var(&key).unwrap_or(default);
    alloc_tea_string(&value)
}

#[no_mangle]
pub extern "C" fn tea_env_has(name: *const TeaString) -> c_int {
    let key = expect_string(name, "env.has expects the name to be a valid UTF-8 string");
    if env::var_os(&key).is_some() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_env_require(name: *const TeaString) -> *mut TeaString {
    let key = expect_string(
        name,
        "env.require expects the name to be a valid UTF-8 string",
    );
    match env::var(&key) {
        Ok(value) => alloc_tea_string(&value),
        Err(_) => panic!("{}", env_error("require", Some(&key), "variable not set")),
    }
}

#[no_mangle]
pub extern "C" fn tea_env_set(name: *const TeaString, value: *const TeaString) {
    let key = expect_string(name, "env.set expects the name to be a valid UTF-8 string");
    let val = expect_string(
        value,
        "env.set expects the value to be a valid UTF-8 string",
    );
    env::set_var(key, val);
}

#[no_mangle]
pub extern "C" fn tea_env_unset(name: *const TeaString) {
    let key = expect_string(
        name,
        "env.unset expects the name to be a valid UTF-8 string",
    );
    env::remove_var(key);
}

#[no_mangle]
pub extern "C" fn tea_env_vars() -> TeaValue {
    let dict = tea_dict_new();
    for (key, value) in env::vars() {
        dict_set_string(dict, &key, &value);
    }
    tea_value_from_dict(dict)
}

#[no_mangle]
pub extern "C" fn tea_env_cwd() -> *mut TeaString {
    match env::current_dir() {
        Ok(path) => alloc_tea_string(&path.to_string_lossy()),
        Err(error) => panic!("{}", env_error("cwd", None, error)),
    }
}

#[no_mangle]
pub extern "C" fn tea_env_set_cwd(path: *const TeaString) {
    let target = expect_path(path);
    env::set_current_dir(&target)
        .unwrap_or_else(|error| panic!("{}", env_error("set_cwd", Some(&target), error)));
}

#[no_mangle]
pub extern "C" fn tea_env_temp_dir() -> *mut TeaString {
    let path = env::temp_dir();
    alloc_tea_string(&path.to_string_lossy())
}

#[no_mangle]
pub extern "C" fn tea_env_home_dir() -> *mut TeaString {
    match home_dir() {
        Some(path) => alloc_tea_string(&path.to_string_lossy()),
        None => alloc_tea_string(""),
    }
}

#[no_mangle]
pub extern "C" fn tea_env_config_dir() -> *mut TeaString {
    match config_dir() {
        Some(path) => alloc_tea_string(&path.to_string_lossy()),
        None => alloc_tea_string(""),
    }
}

unsafe fn tea_value_equals(left: TeaValue, right: TeaValue) -> bool {
    match (left.tag, right.tag) {
        (TeaValueTag::Nil, TeaValueTag::Nil) => true,
        (TeaValueTag::Int, TeaValueTag::Int) => left.payload.int_value == right.payload.int_value,
        (TeaValueTag::Float, TeaValueTag::Float) => {
            left.payload.float_value == right.payload.float_value
        }
        (TeaValueTag::Bool, TeaValueTag::Bool) => {
            left.payload.bool_value == right.payload.bool_value
        }
        (TeaValueTag::String, TeaValueTag::String) => {
            tea_string_to_rust(left.payload.string_value)
                == tea_string_to_rust(right.payload.string_value)
        }
        (TeaValueTag::List, TeaValueTag::List) => {
            left.payload.list_value == right.payload.list_value
        }
        (TeaValueTag::Dict, TeaValueTag::Dict) => {
            left.payload.dict_value == right.payload.dict_value
        }
        (TeaValueTag::Struct, TeaValueTag::Struct) => {
            left.payload.struct_value == right.payload.struct_value
        }
        (TeaValueTag::Closure, TeaValueTag::Closure) => {
            left.payload.closure_value == right.payload.closure_value
        }
        _ => false,
    }
}

unsafe fn tea_value_to_string(value: TeaValue) -> String {
    match value.tag {
        TeaValueTag::Int => format!("{}", value.payload.int_value),
        TeaValueTag::Float => format!("{}", value.payload.float_value),
        TeaValueTag::Bool => {
            if value.payload.bool_value != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        TeaValueTag::Nil => "nil".to_string(),
        TeaValueTag::String => {
            tea_string_to_rust(value.payload.string_value).unwrap_or_else(|| "(null)".to_string())
        }
        TeaValueTag::List => {
            let list_ptr = value.payload.list_value;
            if list_ptr.is_null() {
                return "[]".to_string();
            }
            let list_ref = &*list_ptr;
            if list_ref.len <= 0 {
                return "[]".to_string();
            }
            let mut result = String::from("[");
            for index in 0..list_ref.len {
                if index > 0 {
                    result.push_str(", ");
                }
                let element = *list_ref.items.add(index as usize);
                result.push_str(&tea_value_to_string(element));
            }
            result.push(']');
            result
        }
        TeaValueTag::Dict => {
            let dict_ptr = value.payload.dict_value;
            if dict_ptr.is_null() {
                return "{}".to_string();
            }
            let dict_ref = &*dict_ptr;
            if dict_ref.entries.is_empty() {
                return "{}".to_string();
            }
            let mut result = String::from("{");
            let mut first = true;
            for (key, value) in dict_ref.entries.iter() {
                if !first {
                    result.push_str(", ");
                }
                first = false;
                result.push_str(key);
                result.push_str(": ");
                result.push_str(&tea_value_to_string(*value));
            }
            result.push('}');
            result
        }
        TeaValueTag::Struct => {
            let struct_ptr = value.payload.struct_value;
            if struct_ptr.is_null() {
                return "<struct>".to_string();
            }
            let instance = &*struct_ptr;
            if instance.template.is_null() {
                return "<struct>".to_string();
            }
            let template = &*instance.template;
            let name = tea_cstr_to_rust(template.name).unwrap_or_else(|| "struct".to_string());
            let mut result = String::new();
            result.push_str(&name);
            result.push('(');
            let field_count = template.field_count.max(0) as usize;
            for index in 0..field_count {
                if index > 0 {
                    result.push_str(", ");
                }
                let field_name_ptr = if template.field_names.is_null() {
                    std::ptr::null()
                } else {
                    *template.field_names.add(index)
                };
                let field_name =
                    tea_cstr_to_rust(field_name_ptr).unwrap_or_else(|| format!("field{index}"));
                result.push_str(&field_name);
                result.push_str(": ");
                let field_value = *instance.fields.add(index);
                result.push_str(&tea_value_to_string(field_value));
            }
            result.push(')');
            result
        }
        TeaValueTag::Closure => "<closure>".to_string(),
    }
}

fn tea_value_to_json(value: TeaValue) -> Result<JsonValue, String> {
    unsafe {
        match value.tag {
            TeaValueTag::Nil => Ok(JsonValue::Null),
            TeaValueTag::Bool => Ok(JsonValue::Bool(value.payload.bool_value != 0)),
            TeaValueTag::Int => Ok(JsonValue::Number(serde_json::Number::from(
                value.payload.int_value as i64,
            ))),
            TeaValueTag::Float => {
                let number = value.payload.float_value;
                serde_json::Number::from_f64(number)
                    .map(JsonValue::Number)
                    .ok_or_else(|| "cannot encode NaN or infinite float to JSON".to_string())
            }
            TeaValueTag::String => tea_string_to_rust(value.payload.string_value)
                .map(JsonValue::String)
                .ok_or_else(|| "invalid UTF-8 string value".to_string()),
            TeaValueTag::List => {
                let list_ptr = value.payload.list_value;
                if list_ptr.is_null() {
                    return Ok(JsonValue::Array(Vec::new()));
                }
                let list_ref = &*list_ptr;
                let mut items = Vec::with_capacity(list_ref.len.max(0) as usize);
                for index in 0..list_ref.len.max(0) {
                    let element = *list_ref.items.add(index as usize);
                    items.push(tea_value_to_json(element)?);
                }
                Ok(JsonValue::Array(items))
            }
            TeaValueTag::Dict => {
                let dict_ptr = value.payload.dict_value;
                if dict_ptr.is_null() {
                    return Ok(JsonValue::Object(serde_json::Map::new()));
                }
                let dict_ref = &*dict_ptr;
                let mut object = serde_json::Map::with_capacity(dict_ref.entries.len());
                for (key, entry_value) in dict_ref.entries.iter() {
                    object.insert(key.clone(), tea_value_to_json(*entry_value)?);
                }
                Ok(JsonValue::Object(object))
            }
            TeaValueTag::Struct => {
                let struct_ptr = value.payload.struct_value;
                if struct_ptr.is_null() {
                    return Ok(JsonValue::Object(serde_json::Map::new()));
                }
                let instance = &*struct_ptr;
                if instance.template.is_null() {
                    return Ok(JsonValue::Object(serde_json::Map::new()));
                }
                let template = &*instance.template;
                let mut object =
                    serde_json::Map::with_capacity(template.field_count.max(0) as usize);
                for index in 0..template.field_count.max(0) {
                    let field_name_ptr = if template.field_names.is_null() {
                        std::ptr::null()
                    } else {
                        *template.field_names.add(index as usize)
                    };
                    let field_name =
                        tea_cstr_to_rust(field_name_ptr).unwrap_or_else(|| format!("field{index}"));
                    let field_value = *instance.fields.add(index as usize);
                    object.insert(field_name, tea_value_to_json(field_value)?);
                }
                Ok(JsonValue::Object(object))
            }
            TeaValueTag::Closure => Err("cannot encode closures as JSON".to_string()),
        }
    }
}

fn json_to_tea_value(value: &JsonValue) -> TeaValue {
    match value {
        JsonValue::Null => tea_value_nil(),
        JsonValue::Bool(flag) => tea_value_from_bool(if *flag { 1 } else { 0 }),
        JsonValue::Number(number) => {
            if let Some(int_value) = number.as_i64() {
                tea_value_from_int(int_value)
            } else if let Some(uint_value) = number.as_u64() {
                if uint_value <= i64::MAX as u64 {
                    tea_value_from_int(uint_value as i64)
                } else if let Some(float_value) = number.as_f64() {
                    tea_value_from_float(float_value)
                } else {
                    tea_value_from_float(uint_value as f64)
                }
            } else if let Some(float_value) = number.as_f64() {
                tea_value_from_float(float_value)
            } else {
                tea_value_nil()
            }
        }
        JsonValue::String(text) => {
            let ptr = tea_alloc_string(text.as_ptr() as *const c_char, text.len() as c_longlong);
            tea_value_from_string(ptr)
        }
        JsonValue::Array(items) => {
            let list = tea_alloc_list(items.len() as c_longlong);
            for (index, item) in items.iter().enumerate() {
                tea_list_set(list, index as c_longlong, json_to_tea_value(item));
            }
            tea_value_from_list(list)
        }
        JsonValue::Object(map) => {
            let dict = tea_dict_new();
            for (key, item) in map.iter() {
                let key_ptr =
                    tea_alloc_string(key.as_ptr() as *const c_char, key.len() as c_longlong);
                tea_dict_set(dict, key_ptr, json_to_tea_value(item));
            }
            tea_value_from_dict(dict)
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_struct_template_new(
    name: *const c_char,
    field_count: c_longlong,
    field_names: *const *const c_char,
) -> *mut TeaStructTemplate {
    if field_count < 0 {
        panic!("field count must be non-negative");
    }

    unsafe {
        let count = field_count as usize;
        let mut names: Vec<*const c_char> = Vec::with_capacity(count.max(1));
        for index in 0..count {
            names.push(*field_names.add(index));
        }

        let names_ptr = names.as_ptr();
        std::mem::forget(names);

        Box::into_raw(Box::new(TeaStructTemplate {
            name,
            field_count,
            field_names: names_ptr,
        }))
    }
}

#[no_mangle]
pub extern "C" fn tea_struct_template_field_name(
    template: *const TeaStructTemplate,
    index: c_longlong,
) -> *const c_char {
    unsafe {
        if template.is_null() {
            panic!("null struct template");
        }
        if index < 0 || index >= (*template).field_count {
            panic!("field index out of bounds");
        }
        *(*template).field_names.add(index as usize)
    }
}

#[no_mangle]
pub extern "C" fn tea_alloc_struct(template: *const TeaStructTemplate) -> *mut TeaStructInstance {
    unsafe {
        if template.is_null() {
            panic!("null struct template");
        }
        let template_ref = &*template;
        if template_ref.field_count < 0 {
            panic!("field count must be non-negative");
        }
        let count = template_ref.field_count as usize;
        let mut fields = Vec::with_capacity(count.max(1));
        fields.resize_with(count, || tea_value_nil());

        let instance = TeaStructInstance {
            template,
            fields: fields.as_mut_ptr(),
        };

        let raw = Box::into_raw(Box::new(instance));
        std::mem::forget(fields);
        raw
    }
}

#[no_mangle]
pub extern "C" fn tea_struct_set_field(
    instance: *mut TeaStructInstance,
    index: c_longlong,
    value: TeaValue,
) {
    unsafe {
        if instance.is_null() {
            panic!("null struct instance");
        }
        let instance_ref = &mut *instance;
        if instance_ref.template.is_null() {
            panic!("struct instance has null template");
        }
        let template_ref = &*instance_ref.template;
        if index < 0 || index >= template_ref.field_count {
            panic!("field index out of bounds");
        }
        *instance_ref.fields.add(index as usize) = value;
    }
}

#[no_mangle]
pub extern "C" fn tea_struct_get_field(
    instance: *const TeaStructInstance,
    index: c_longlong,
) -> TeaValue {
    unsafe {
        if instance.is_null() {
            panic!("null struct instance");
        }
        let instance_ref = &*instance;
        if instance_ref.template.is_null() {
            panic!("struct instance has null template");
        }
        let template_ref = &*instance_ref.template;
        if index < 0 || index >= template_ref.field_count {
            panic!("field index out of bounds");
        }
        *instance_ref.fields.add(index as usize)
    }
}

#[no_mangle]
pub extern "C" fn tea_struct_equal(
    left: *const TeaStructInstance,
    right: *const TeaStructInstance,
) -> c_int {
    if left == right {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_print_struct(instance: *const TeaStructInstance) {
    unsafe {
        if instance.is_null() {
            println!("<struct nil>");
            return;
        }
        let instance_ref = &*instance;
        if instance_ref.template.is_null() {
            println!("<struct ?>");
            return;
        }
        let template_ref = &*instance_ref.template;
        let struct_name = if template_ref.name.is_null() {
            "<anonymous>"
        } else {
            CStr::from_ptr(template_ref.name)
                .to_str()
                .unwrap_or("<invalid utf8>")
        };

        print!("{struct_name}(");
        for i in 0..template_ref.field_count {
            if i > 0 {
                print!(", ");
            }
            let field_name_ptr = tea_struct_template_field_name(instance_ref.template, i);
            let field_name = if field_name_ptr.is_null() {
                "<field>"
            } else {
                CStr::from_ptr(field_name_ptr)
                    .to_str()
                    .unwrap_or("<invalid utf8>")
            };
            print!("{field_name}: ");
            let value = *instance_ref.fields.add(i as usize);
            print_value(value);
        }
        println!(")");
    }
}

#[no_mangle]
pub extern "C" fn tea_print_closure(closure: *const TeaClosure) {
    if closure.is_null() {
        println!("<closure nil>");
    } else {
        println!("<closure>");
    }
}

#[no_mangle]
pub extern "C" fn tea_closure_new(
    function: *const c_void,
    capture_count: c_longlong,
) -> *mut TeaClosure {
    if capture_count < 0 {
        panic!("capture count must be non-negative");
    }

    let count = capture_count as usize;
    let mut captures = Vec::with_capacity(count.max(1));
    captures.resize_with(count, || tea_value_nil());

    let closure = TeaClosure {
        function,
        captures: captures.as_mut_ptr(),
        capture_count,
    };

    let raw = Box::into_raw(Box::new(closure));
    std::mem::forget(captures);
    raw
}

#[no_mangle]
pub extern "C" fn tea_closure_set_capture(
    closure: *mut TeaClosure,
    index: c_longlong,
    value: TeaValue,
) {
    unsafe {
        if closure.is_null() {
            panic!("null closure");
        }
        let closure_ref = &mut *closure;
        if index < 0 || index >= closure_ref.capture_count {
            panic!("capture index out of bounds");
        }
        *closure_ref.captures.add(index as usize) = value;
    }
}

#[no_mangle]
pub extern "C" fn tea_closure_get_capture(
    closure: *const TeaClosure,
    index: c_longlong,
) -> TeaValue {
    unsafe {
        if closure.is_null() {
            panic!("null closure");
        }
        let closure_ref = &*closure;
        if index < 0 || index >= closure_ref.capture_count {
            panic!("capture index out of bounds");
        }
        *closure_ref.captures.add(index as usize)
    }
}

#[no_mangle]
pub extern "C" fn tea_closure_equal(left: *const TeaClosure, right: *const TeaClosure) -> c_int {
    if left == right {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_assert(condition: c_int, message: *const TeaString) {
    if condition != 0 {
        return;
    }
    let msg = tea_string_to_rust(message).unwrap_or_else(|| "assertion failed".to_string());
    panic!("{msg}");
}

#[no_mangle]
pub extern "C" fn tea_assert_eq(left: TeaValue, right: TeaValue) {
    unsafe {
        if !tea_value_equals(left, right) {
            let left_str = tea_value_to_string(left);
            let right_str = tea_value_to_string(right);
            panic!("assert_eq failed: left {} != right {}", left_str, right_str);
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_assert_ne(left: TeaValue, right: TeaValue) {
    unsafe {
        if tea_value_equals(left, right) {
            let value_str = tea_value_to_string(left);
            panic!("assert_ne failed: values are both {}", value_str);
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_fail(message: *const TeaString) {
    let msg = tea_string_to_rust(message).unwrap_or_else(|| "fail called".to_string());
    panic!("{msg}");
}

#[no_mangle]
pub extern "C" fn tea_util_len(value: TeaValue) -> c_longlong {
    unsafe {
        match value.tag {
            TeaValueTag::String => tea_string_to_rust(value.payload.string_value)
                .map(|s| s.chars().count() as c_longlong)
                .unwrap_or(0),
            TeaValueTag::List => {
                let list_ptr = value.payload.list_value;
                if list_ptr.is_null() {
                    0
                } else {
                    (*list_ptr).len
                }
            }
            _ => panic!("len builtin expects a String or List"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_util_to_string(value: TeaValue) -> *mut TeaString {
    unsafe {
        let text = tea_value_to_string(value);
        let bytes = text.into_bytes();
        tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong)
    }
}

#[no_mangle]
pub extern "C" fn tea_util_clamp_int(
    value: c_longlong,
    min: c_longlong,
    max: c_longlong,
) -> c_longlong {
    if min > max {
        panic!("clamp_int expects minimum <= maximum");
    }
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_nil(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::Nil) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_bool(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::Bool) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_int(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::Int) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_float(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::Float) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_string(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::String) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_list(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::List) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_util_is_struct(value: TeaValue) -> c_int {
    if matches!(value.tag, TeaValueTag::Struct) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_read_text(path: *const TeaString) -> *mut TeaString {
    let path_str = expect_path(path);
    let contents = fs::read_to_string(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("read_text", &path_str, &error)));
    let bytes = contents.as_bytes();
    tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong)
}

#[no_mangle]
pub extern "C" fn tea_fs_write_text(path: *const TeaString, contents: *const TeaString) {
    let path_str = expect_path(path);
    let text = expect_string(
        contents,
        "write_text expects the contents argument to be a valid string",
    );
    fs::write(&path_str, text.as_bytes())
        .unwrap_or_else(|error| panic!("{}", fs_error("write_text", &path_str, &error)));
}

#[no_mangle]
pub extern "C" fn tea_fs_write_text_atomic(path: *const TeaString, contents: *const TeaString) {
    let path_str = expect_path(path);
    let text = expect_string(
        contents,
        "write_text_atomic expects the contents argument to be a valid string",
    );
    let fs_path = Path::new(&path_str);
    write_atomic_bytes(fs_path, text.as_bytes()).unwrap_or_else(|error| {
        panic!("{}", fs_error("write_text_atomic", &path_str, &error));
    });
}

#[no_mangle]
pub extern "C" fn tea_fs_read_bytes(path: *const TeaString) -> *mut TeaList {
    let path_str = expect_path(path);
    let bytes = fs::read(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("read_bytes", &path_str, &error)));
    let list = tea_alloc_list(bytes.len() as c_longlong);
    for (index, byte) in bytes.into_iter().enumerate() {
        tea_list_set(
            list,
            index as c_longlong,
            tea_value_from_int(byte as c_longlong),
        );
    }
    list
}

#[no_mangle]
pub extern "C" fn tea_fs_write_bytes(path: *const TeaString, data: *const TeaList) {
    if data.is_null() {
        panic!("write_bytes expects a List argument");
    }
    let path_str = expect_path(path);
    let list_ref = unsafe { &*data };
    let mut buffer = Vec::with_capacity(list_ref.len as usize);
    for index in 0..list_ref.len {
        let value = tea_list_get(data, index);
        let byte = tea_value_as_int(value);
        if byte < 0 || byte > 255 {
            panic!("write_bytes expects values between 0 and 255");
        }
        buffer.push(byte as u8);
    }
    fs::write(&path_str, buffer)
        .unwrap_or_else(|error| panic!("{}", fs_error("write_bytes", &path_str, &error)));
}

#[no_mangle]
pub extern "C" fn tea_fs_write_bytes_atomic(path: *const TeaString, data: *const TeaList) {
    if data.is_null() {
        panic!("write_bytes_atomic expects a List argument");
    }
    let path_str = expect_path(path);
    let list_ref = unsafe { &*data };
    let mut buffer = Vec::with_capacity(list_ref.len as usize);
    for index in 0..list_ref.len {
        let value = tea_list_get(data, index);
        let byte = tea_value_as_int(value);
        if byte < 0 || byte > 255 {
            panic!("write_bytes_atomic expects values between 0 and 255");
        }
        buffer.push(byte as u8);
    }
    let fs_path = Path::new(&path_str);
    write_atomic_bytes(fs_path, &buffer).unwrap_or_else(|error| {
        panic!("{}", fs_error("write_bytes_atomic", &path_str, &error));
    });
}

#[no_mangle]
pub extern "C" fn tea_fs_create_dir(path: *const TeaString, recursive: c_int) {
    let path_str = expect_path(path);
    if recursive != 0 {
        fs::create_dir_all(&path_str)
            .unwrap_or_else(|error| panic!("{}", fs_error("create_dir", &path_str, &error)));
    } else {
        fs::create_dir(&path_str)
            .unwrap_or_else(|error| panic!("{}", fs_error("create_dir", &path_str, &error)));
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_ensure_dir(path: *const TeaString) {
    let path_str = expect_path(path);
    fs::create_dir_all(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("ensure_dir", &path_str, &error)));
}

#[no_mangle]
pub extern "C" fn tea_fs_ensure_parent(path: *const TeaString) {
    let path_str = expect_path(path);
    let fs_path = Path::new(&path_str);
    if let Some(parent) = fs_path.parent() {
        if parent.as_os_str().is_empty() {
            return;
        }
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!("{}", fs_error("ensure_parent", &path_str, &error));
        });
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_remove(path: *const TeaString) {
    let path_str = expect_path(path);
    let fs_path = Path::new(&path_str);
    if fs_path.is_dir() {
        fs::remove_dir_all(fs_path)
            .unwrap_or_else(|error| panic!("{}", fs_error("remove", &path_str, &error)));
    } else {
        fs::remove_file(fs_path)
            .unwrap_or_else(|error| panic!("{}", fs_error("remove", &path_str, &error)));
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_exists(path: *const TeaString) -> c_int {
    let path_str = expect_path(path);
    if Path::new(&path_str).exists() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_is_dir(path: *const TeaString) -> c_int {
    let path_str = expect_path(path);
    if Path::new(&path_str).is_dir() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_is_symlink(path: *const TeaString) -> c_int {
    let path_str = expect_path(path);
    match fs::symlink_metadata(&path_str) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                1
            } else {
                0
            }
        }
        Err(error) => panic!("{}", fs_error("is_symlink", &path_str, &error)),
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_size(path: *const TeaString) -> c_longlong {
    let path_str = expect_path(path);
    fs::metadata(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("size", &path_str, &error)))
        .len() as c_longlong
}

#[no_mangle]
pub extern "C" fn tea_fs_modified(path: *const TeaString) -> c_longlong {
    let path_str = expect_path(path);
    let metadata = fs::metadata(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("modified", &path_str, &error)));
    let modified = metadata
        .modified()
        .unwrap_or_else(|error| panic!("{}", fs_error("modified", &path_str, &error)));
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as c_longlong,
        Err(error) => panic!("modified time for '{path_str}' precedes Unix epoch: {error}"),
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_permissions(path: *const TeaString) -> c_longlong {
    let path_str = expect_path(path);
    let metadata = fs::symlink_metadata(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("permissions", &path_str, &error)));
    metadata_mode(&metadata) as c_longlong
}

#[no_mangle]
pub extern "C" fn tea_fs_is_readonly(path: *const TeaString) -> c_int {
    let path_str = expect_path(path);
    let metadata = fs::metadata(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("is_readonly", &path_str, &error)));
    if metadata.permissions().readonly() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_fs_list_dir(path: *const TeaString) -> *mut TeaList {
    let path_str = expect_path(path);
    let mut entries = Vec::new();
    let dir = fs::read_dir(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("list_dir", &path_str, &error)));
    for entry in dir {
        match entry {
            Ok(dir_entry) => {
                let entry_path = dir_entry.path();
                let display = entry_path.to_string_lossy().into_owned();
                entries.push(display);
            }
            Err(error) => panic!("{}", fs_error("list_dir", &path_str, &error)),
        }
    }
    entries.sort();
    strings_to_list(entries)
}

#[no_mangle]
pub extern "C" fn tea_fs_walk(path: *const TeaString) -> *mut TeaList {
    let path_str = expect_path(path);
    let mut entries = Vec::new();
    for entry in WalkDir::new(&path_str) {
        match entry {
            Ok(dir_entry) => {
                if dir_entry.depth() == 0 {
                    continue;
                }
                let entry_path = dir_entry.path();
                let display = entry_path.to_string_lossy().into_owned();
                entries.push(display);
            }
            Err(error) => panic!("{}", fs_error("walk", &path_str, &error)),
        }
    }
    entries.sort();
    strings_to_list(entries)
}

#[no_mangle]
pub extern "C" fn tea_fs_glob(pattern: *const TeaString) -> *mut TeaList {
    let pattern_str = expect_path(pattern);
    let mut matches = Vec::new();
    match glob(&pattern_str) {
        Ok(paths) => {
            for path in paths {
                match path {
                    Ok(entry) => matches.push(entry.to_string_lossy().into_owned()),
                    Err(error) => panic!("{}", fs_error("glob", &pattern_str, &error)),
                }
            }
        }
        Err(error) => panic!("{}", fs_error("glob", &pattern_str, &error)),
    }
    matches.sort();
    strings_to_list(matches)
}

#[no_mangle]
pub extern "C" fn tea_fs_metadata(path: *const TeaString) -> TeaValue {
    let path_str = expect_path(path);
    let fs_path = PathBuf::from(&path_str);
    let metadata = fs::symlink_metadata(&fs_path)
        .unwrap_or_else(|error| panic!("{}", fs_error("metadata", &path_str, &error)));

    let dict = tea_dict_new();
    dict_set_string(dict, "path", &path_str);
    dict_set_bool(dict, "is_dir", metadata.is_dir());
    dict_set_bool(dict, "is_file", metadata.is_file());
    dict_set_bool(dict, "is_symlink", metadata.file_type().is_symlink());
    dict_set_bool(dict, "readonly", metadata.permissions().readonly());
    dict_set_int(dict, "size", metadata.len() as i64);
    dict_set_int(dict, "permissions", metadata_mode(&metadata));

    let modified = metadata.modified().ok().and_then(|time| {
        time.duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs() as i64)
    });
    dict_set_optional_int(dict, "modified", modified);

    if let Some(parent) = fs_path.parent() {
        let parent_str = parent.to_string_lossy();
        dict_set_string(dict, "parent", parent_str.as_ref());
    } else {
        dict_set_value(dict, "parent", tea_value_nil());
    }

    tea_value_from_dict(dict)
}

#[no_mangle]
pub extern "C" fn tea_fs_open_read(path: *const TeaString) -> c_longlong {
    let path_str = expect_path(path);
    let file = File::open(&path_str)
        .unwrap_or_else(|error| panic!("{}", fs_error("open_read", &path_str, &error)));
    let mut table = fs_handles().lock().unwrap();
    let id = NEXT_FS_HANDLE.fetch_add(1, Ordering::SeqCst);
    table.insert(
        id,
        FsHandle {
            reader: BufReader::new(file),
        },
    );
    id
}

#[no_mangle]
pub extern "C" fn tea_fs_read_chunk(handle: c_longlong, size: c_longlong) -> *mut TeaList {
    if size <= 0 {
        panic!("read_chunk expects a positive chunk size");
    }
    let mut table = fs_handles().lock().unwrap();
    let entry = table
        .get_mut(&(handle as i64))
        .unwrap_or_else(|| panic!("invalid file handle {handle}"));
    let mut buffer = vec![0u8; size as usize];
    let bytes_read = entry.reader.read(&mut buffer).unwrap_or_else(|error| {
        let target = format!("handle {handle}");
        panic!("{}", fs_error("read_chunk", &target, &error));
    });
    buffer.truncate(bytes_read);
    drop(table);
    let list = tea_alloc_list(bytes_read as c_longlong);
    for (index, byte) in buffer.into_iter().enumerate() {
        tea_list_set(
            list,
            index as c_longlong,
            tea_value_from_int(byte as c_longlong),
        );
    }
    list
}

#[no_mangle]
pub extern "C" fn tea_fs_close(handle: c_longlong) {
    let mut table = fs_handles().lock().unwrap();
    if table.remove(&(handle as i64)).is_none() {
        panic!("invalid file handle {handle}");
    }
}

#[no_mangle]
pub extern "C" fn tea_alloc_string(ptr: *const c_char, len: c_longlong) -> *mut TeaString {
    unsafe {
        let bytes = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        let mut buffer = Vec::with_capacity(bytes.len() + 1);
        buffer.extend_from_slice(bytes);
        buffer.push(0);
        let data_ptr = buffer.as_ptr() as *const c_char;
        std::mem::forget(buffer);
        Box::into_raw(Box::new(TeaString {
            len,
            data: data_ptr,
        }))
    }
}

#[no_mangle]
pub extern "C" fn tea_alloc_list(len: c_longlong) -> *mut TeaList {
    let capacity = len.max(4);
    let mut items = Vec::with_capacity(capacity as usize);
    for _ in 0..capacity {
        items.push(tea_value_nil());
    }
    let list = TeaList {
        len,
        capacity,
        items: items.as_mut_ptr(),
    };
    let raw = Box::into_raw(Box::new(list));
    std::mem::forget(items);
    raw
}

#[no_mangle]
pub extern "C" fn tea_list_set(list: *mut TeaList, index: c_longlong, value: TeaValue) {
    unsafe {
        if list.is_null() {
            return;
        }
        let list_ref = &mut *list;
        if index < 0 || index >= list_ref.len {
            panic!("index out of bounds");
        }
        *list_ref.items.add(index as usize) = value;
    }
}

#[no_mangle]
pub extern "C" fn tea_list_get(list: *const TeaList, index: c_longlong) -> TeaValue {
    unsafe {
        if list.is_null() {
            panic!("null list");
        }
        let list_ref = &*list;
        if index < 0 || index >= list_ref.len {
            panic!("index out of bounds");
        }
        *list_ref.items.add(index as usize)
    }
}

#[no_mangle]
pub extern "C" fn tea_dict_new() -> *mut TeaDict {
    Box::into_raw(Box::new(TeaDict {
        entries: HashMap::new(),
    }))
}

#[no_mangle]
pub extern "C" fn tea_dict_set(dict: *mut TeaDict, key: *const TeaString, value: TeaValue) {
    if dict.is_null() {
        panic!("null dict");
    }
    let key_str = expect_string(key, "dict key must be a valid string");
    unsafe {
        let dict_ref = &mut *dict;
        dict_ref.entries.insert(key_str, value);
    }
}

#[no_mangle]
pub extern "C" fn tea_dict_get(dict: *const TeaDict, key: *const TeaString) -> TeaValue {
    if dict.is_null() {
        panic!("null dict");
    }
    let key_str = expect_string(key, "dict key must be a valid string");
    unsafe {
        let dict_ref = &*dict;
        dict_ref
            .entries
            .get(&key_str)
            .copied()
            .unwrap_or_else(|| tea_value_nil())
    }
}

#[no_mangle]
pub extern "C" fn tea_dict_len(dict: *const TeaDict) -> c_longlong {
    if dict.is_null() {
        return 0;
    }
    unsafe { (&*dict).entries.len() as c_longlong }
}

#[no_mangle]
pub extern "C" fn tea_dict_equal(left: *const TeaDict, right: *const TeaDict) -> c_int {
    if left == right {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_io_read_line() -> TeaValue {
    let mut buffer = String::new();
    match std::io::stdin().read_line(&mut buffer) {
        Ok(0) => tea_value_nil(),
        Ok(_) => {
            while buffer.ends_with(['\n', '\r']) {
                buffer.pop();
            }
            let bytes = buffer.as_bytes();
            let string_ptr =
                tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong);
            tea_value_from_string(string_ptr)
        }
        Err(error) => panic!("{}", io_error("read_line", &error)),
    }
}

#[no_mangle]
pub extern "C" fn tea_io_read_all() -> *mut TeaString {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .unwrap_or_else(|error| panic!("{}", io_error("read_all", &error)));
    let bytes = buffer.as_bytes();
    tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong)
}

#[no_mangle]
pub extern "C" fn tea_io_read_bytes() -> *mut TeaList {
    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .unwrap_or_else(|error| panic!("{}", io_error("read_bytes", &error)));
    let list = tea_alloc_list(buffer.len() as c_longlong);
    for (index, byte) in buffer.into_iter().enumerate() {
        tea_list_set(
            list,
            index as c_longlong,
            tea_value_from_int(byte as c_longlong),
        );
    }
    list
}

#[no_mangle]
pub extern "C" fn tea_io_write(text: *const TeaString) {
    let data = expect_string(text, "write expects a valid string argument");
    std::io::stdout()
        .write_all(data.as_bytes())
        .unwrap_or_else(|error| panic!("{}", io_error("write", &error)));
}

#[no_mangle]
pub extern "C" fn tea_io_write_err(text: *const TeaString) {
    let data = expect_string(text, "write_err expects a valid string argument");
    std::io::stderr()
        .write_all(data.as_bytes())
        .unwrap_or_else(|error| panic!("{}", io_error("write_err", &error)));
}

#[no_mangle]
pub extern "C" fn tea_io_flush() {
    std::io::stdout()
        .flush()
        .unwrap_or_else(|error| panic!("{}", io_error("flush", &error)));
}

#[no_mangle]
pub extern "C" fn tea_json_encode(value: TeaValue) -> *mut TeaString {
    let json_value =
        tea_value_to_json(value).unwrap_or_else(|error| panic!("failed to encode JSON: {error}"));
    let encoded = serde_json::to_string(&json_value)
        .unwrap_or_else(|error| panic!("failed to encode JSON: {error}"));
    let bytes = encoded.as_bytes();
    tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong)
}

#[no_mangle]
pub extern "C" fn tea_json_decode(text: *const TeaString) -> TeaValue {
    let input = expect_string(text, "json.decode expects a String argument");
    let parsed: JsonValue = serde_json::from_str(&input)
        .unwrap_or_else(|error| panic!("failed to decode JSON: {error}"));
    json_to_tea_value(&parsed)
}

#[no_mangle]
pub extern "C" fn tea_yaml_encode(value: TeaValue) -> *mut TeaString {
    let json_value =
        tea_value_to_json(value).unwrap_or_else(|error| panic!("failed to encode YAML: {error}"));
    let yaml_value = serde_yaml::to_string(&json_value)
        .unwrap_or_else(|error| panic!("failed to encode YAML: {error}"));
    let bytes = yaml_value.as_bytes();
    tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong)
}

#[no_mangle]
pub extern "C" fn tea_yaml_decode(text: *const TeaString) -> TeaValue {
    let input = expect_string(text, "yaml.decode expects a String argument");
    let parsed: YamlValue = serde_yaml::from_str(&input)
        .unwrap_or_else(|error| panic!("failed to decode YAML: {error}"));
    let json_value = serde_json::to_value(parsed)
        .unwrap_or_else(|error| panic!("failed to normalise YAML: {error}"));
    json_to_tea_value(&json_value)
}

fn collect_cli_args() -> Vec<String> {
    env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn detect_program_name() -> Option<String> {
    env::args_os().next().map(|arg| {
        let path = Path::new(&arg);
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| arg.to_string_lossy().into_owned())
    })
}

fn tea_list_to_runtime(list: *const TeaList) -> Result<Vec<RuntimeValue>> {
    if list.is_null() {
        return Ok(Vec::new());
    }
    unsafe {
        let list_ref = &*list;
        let mut values = Vec::with_capacity(list_ref.len as usize);
        for index in 0..list_ref.len {
            let value = *list_ref.items.add(index as usize);
            values.push(runtime_value_from_tea(value)?);
        }
        Ok(values)
    }
}

fn tea_dict_to_runtime(dict: *const TeaDict) -> Result<HashMap<String, RuntimeValue>> {
    if dict.is_null() {
        return Ok(HashMap::new());
    }
    unsafe {
        let dict_ref = &*dict;
        let mut map = HashMap::with_capacity(dict_ref.entries.len());
        for (key, value) in dict_ref.entries.iter() {
            map.insert(key.clone(), runtime_value_from_tea(*value)?);
        }
        Ok(map)
    }
}

fn runtime_value_from_tea(value: TeaValue) -> Result<RuntimeValue> {
    unsafe {
        match value.tag {
            TeaValueTag::Nil => Ok(RuntimeValue::Nil),
            TeaValueTag::Int => Ok(RuntimeValue::Int(value.payload.int_value)),
            TeaValueTag::Float => Ok(RuntimeValue::Float(value.payload.float_value)),
            TeaValueTag::Bool => Ok(RuntimeValue::Bool(value.payload.bool_value != 0)),
            TeaValueTag::String => Ok(RuntimeValue::String(expect_string(
                value.payload.string_value,
                "cli value expects a valid string",
            ))),
            TeaValueTag::List => {
                let items = tea_list_to_runtime(value.payload.list_value)?;
                Ok(RuntimeValue::List(items))
            }
            TeaValueTag::Dict => {
                let map = tea_dict_to_runtime(value.payload.dict_value)?;
                Ok(RuntimeValue::Dict(map))
            }
            TeaValueTag::Struct | TeaValueTag::Closure => {
                Err(anyhow!("cli spec does not support struct values"))
            }
        }
    }
}

fn runtime_list_to_tea(items: &[RuntimeValue]) -> Result<TeaValue> {
    let list = tea_alloc_list(items.len() as c_longlong);
    for (index, item) in items.iter().enumerate() {
        let value = runtime_value_to_tea(item)?;
        tea_list_set(list, index as c_longlong, value);
    }
    Ok(tea_value_from_list(list))
}

fn runtime_dict_to_tea(map: &HashMap<String, RuntimeValue>) -> Result<TeaValue> {
    let dict = tea_dict_new();
    for (key, value) in map {
        let value_tea = runtime_value_to_tea(value)?;
        let bytes = key.as_bytes();
        let key_ptr = tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong);
        tea_dict_set(dict, key_ptr, value_tea);
    }
    Ok(tea_value_from_dict(dict))
}

fn runtime_value_to_tea(value: &RuntimeValue) -> Result<TeaValue> {
    match value {
        RuntimeValue::Nil => Ok(tea_value_nil()),
        RuntimeValue::Int(v) => Ok(tea_value_from_int(*v as c_longlong)),
        RuntimeValue::Float(v) => Ok(tea_value_from_float(*v as c_double)),
        RuntimeValue::Bool(v) => Ok(tea_value_from_bool(if *v { 1 } else { 0 })),
        RuntimeValue::String(text) => {
            let bytes = text.as_bytes();
            let ptr = tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong);
            Ok(tea_value_from_string(ptr))
        }
        RuntimeValue::List(items) => runtime_list_to_tea(items),
        RuntimeValue::Dict(map) => runtime_dict_to_tea(map),
    }
}

fn runtime_strings_to_tea(items: &[String]) -> Result<TeaValue> {
    let runtime_items = items
        .iter()
        .cloned()
        .map(RuntimeValue::String)
        .collect::<Vec<_>>();
    runtime_list_to_tea(&runtime_items)
}

fn runtime_scope_to_tea(scope: &CliScopeOutcome) -> Result<TeaValue> {
    let dict = tea_dict_new();

    let name_value = runtime_value_to_tea(&RuntimeValue::String(scope.name.clone()))?;
    let name_bytes = scope.name.as_bytes();
    let name_key = tea_alloc_string(
        name_bytes.as_ptr() as *const c_char,
        name_bytes.len() as c_longlong,
    );
    tea_dict_set(dict, name_key, name_value);

    let options_value = runtime_dict_to_tea(&scope.options)?;
    let options_key_bytes = b"options";
    let options_key = tea_alloc_string(
        options_key_bytes.as_ptr() as *const c_char,
        options_key_bytes.len() as c_longlong,
    );
    tea_dict_set(dict, options_key, options_value);

    let positionals_value = runtime_dict_to_tea(&scope.positionals)?;
    let positionals_key_bytes = b"positionals";
    let positionals_key = tea_alloc_string(
        positionals_key_bytes.as_ptr() as *const c_char,
        positionals_key_bytes.len() as c_longlong,
    );
    tea_dict_set(dict, positionals_key, positionals_value);

    Ok(tea_value_from_dict(dict))
}

fn runtime_scopes_to_tea(scopes: &[CliScopeOutcome]) -> Result<TeaValue> {
    let list = tea_alloc_list(scopes.len() as c_longlong);
    for (index, scope) in scopes.iter().enumerate() {
        let value = runtime_scope_to_tea(scope)?;
        tea_list_set(list, index as c_longlong, value);
    }
    Ok(tea_value_from_list(list))
}

fn cli_outcome_to_struct(
    template: *const TeaStructTemplate,
    outcome: &CliParseOutcome,
) -> Result<*mut TeaStructInstance> {
    if template.is_null() {
        return Err(anyhow!("cli.parse requires a valid struct template"));
    }
    let instance = tea_alloc_struct(template);
    if instance.is_null() {
        return Err(anyhow!("failed to allocate cli parse result struct"));
    }
    unsafe {
        let struct_ref = &mut *instance;
        let fields = std::slice::from_raw_parts_mut(struct_ref.fields, 10);

        fields[0] = tea_value_from_bool(if outcome.ok { 1 } else { 0 });
        fields[1] = tea_value_from_int(outcome.exit as c_longlong);
        fields[2] = runtime_value_to_tea(&RuntimeValue::String(outcome.command.clone()))?;
        fields[3] = runtime_strings_to_tea(&outcome.path)?;
        fields[4] = runtime_dict_to_tea(&outcome.options)?;
        fields[5] = runtime_dict_to_tea(&outcome.positionals)?;
        fields[6] = runtime_scopes_to_tea(&outcome.scopes)?;
        fields[7] = runtime_strings_to_tea(&outcome.rest)?;
        fields[8] = runtime_value_to_tea(&RuntimeValue::String(outcome.message.clone()))?;
        fields[9] = runtime_value_to_tea(&RuntimeValue::String(outcome.help.clone()))?;
    }

    Ok(instance)
}

fn tea_value_list_to_strings(value: TeaValue) -> Result<Vec<String>> {
    unsafe {
        match value.tag {
            TeaValueTag::Nil => Ok(Vec::new()),
            TeaValueTag::List => {
                let list = value.payload.list_value;
                let list_ref = &*list;
                let mut strings = Vec::with_capacity(list_ref.len as usize);
                for index in 0..list_ref.len {
                    let element = *list_ref.items.add(index as usize);
                    match element.tag {
                        TeaValueTag::String => {
                            strings.push(expect_string(
                                element.payload.string_value,
                                "cli override expects String values",
                            ));
                        }
                        _ => {
                            return Err(anyhow!("cli override expects a List of Strings"));
                        }
                    }
                }
                Ok(strings)
            }
            _ => Err(anyhow!("cli override expects a List of Strings")),
        }
    }
}

fn tea_value_dict_to_string_map(value: TeaValue) -> Result<HashMap<String, String>> {
    match runtime_value_from_tea(value)? {
        RuntimeValue::Nil => Ok(HashMap::new()),
        RuntimeValue::Dict(map) => {
            let mut result = HashMap::with_capacity(map.len());
            for (key, value) in map {
                match value {
                    RuntimeValue::String(text) => {
                        result.insert(key, text);
                    }
                    _ => return Err(anyhow!("process env expects String values")),
                }
            }
            Ok(result)
        }
        _ => Err(anyhow!("process env expects a Dict[String, String]")),
    }
}

fn read_process_pipe<R: Read>(
    reader: &mut Option<BufReader<R>>,
    size: Option<usize>,
) -> std::io::Result<String> {
    if let Some(ref mut handle) = reader {
        let mut buffer = Vec::new();
        if let Some(limit) = size {
            let mut limited = handle.take(limit as u64);
            limited.read_to_end(&mut buffer)?;
        } else {
            handle.read_to_end(&mut buffer)?;
        }
        Ok(String::from_utf8_lossy(&buffer).to_string())
    } else {
        Ok(String::new())
    }
}

fn build_process_result_struct(
    template: *const TeaStructTemplate,
    exit: i64,
    stdout: String,
    stderr: String,
    command: String,
) -> Result<*mut TeaStructInstance> {
    if template.is_null() {
        return Err(anyhow!("process result template is null"));
    }
    let instance = tea_alloc_struct(template);
    if instance.is_null() {
        return Err(anyhow!("failed to allocate process result struct"));
    }
    unsafe {
        let struct_ref = &mut *instance;
        let fields = std::slice::from_raw_parts_mut(struct_ref.fields, 5);
        fields[0] = tea_value_from_int(exit as c_longlong);
        fields[1] = tea_value_from_bool(if exit == 0 { 1 } else { 0 });
        let stdout_ptr = alloc_tea_string(&stdout);
        fields[2] = tea_value_from_string(stdout_ptr);
        let stderr_ptr = alloc_tea_string(&stderr);
        fields[3] = tea_value_from_string(stderr_ptr);
        let command_ptr = alloc_tea_string(&command);
        fields[4] = tea_value_from_string(command_ptr);
    }
    Ok(instance)
}

#[no_mangle]
pub extern "C" fn tea_cli_args() -> *mut TeaList {
    let args = collect_cli_args();
    let list = tea_alloc_list(args.len() as c_longlong);
    for (index, arg) in args.iter().enumerate() {
        let bytes = arg.as_bytes();
        let string_ptr =
            tea_alloc_string(bytes.as_ptr() as *const c_char, bytes.len() as c_longlong);
        tea_list_set(list, index as c_longlong, tea_value_from_string(string_ptr));
    }
    list
}

#[no_mangle]
pub extern "C" fn tea_cli_parse(
    template: *const TeaStructTemplate,
    spec: TeaValue,
    override_args: TeaValue,
) -> *mut TeaStructInstance {
    let spec_runtime = runtime_value_from_tea(spec)
        .unwrap_or_else(|error| panic!("{}", cli_error("parse", &error)));

    let args_override = match override_args.tag {
        TeaValueTag::Nil => None,
        TeaValueTag::List => Some(
            tea_value_list_to_strings(override_args)
                .unwrap_or_else(|error| panic!("{}", cli_error("parse", &error))),
        ),
        _ => panic!("cli.parse override expects a List or nil"),
    };

    let args = args_override.unwrap_or_else(collect_cli_args);
    let program_name = detect_program_name();

    let outcome = crate::cli::parse_cli(&spec_runtime, &args, program_name.as_deref())
        .unwrap_or_else(|error| panic!("{}", cli_error("parse", &error)));

    cli_outcome_to_struct(template, &outcome)
        .unwrap_or_else(|error| panic!("{}", cli_error("parse", &error)))
}

#[no_mangle]
pub extern "C" fn tea_process_run(
    template: *const TeaStructTemplate,
    command: *const TeaString,
    args: TeaValue,
    env: TeaValue,
    cwd: TeaValue,
    stdin_value: TeaValue,
) -> *mut TeaStructInstance {
    let command_str = expect_string(command, "process.run expects a valid command string");
    let arguments = tea_value_list_to_strings(args)
        .unwrap_or_else(|error| panic!("{}", process_error("run", &command_str, error)));
    let env_map = tea_value_dict_to_string_map(env)
        .unwrap_or_else(|error| panic!("{}", process_error("run", &command_str, error)));
    let cwd_str = match cwd.tag {
        TeaValueTag::Nil => None,
        TeaValueTag::String => Some(unsafe {
            expect_string(
                cwd.payload.string_value,
                "process.run expects cwd to be a valid string",
            )
        }),
        // Gracefully ignore unexpected value kinds (seen when LLVM lowers nil
        // incorrectly) so we fall back to the current working directory instead
        // of aborting the process.
        _ => None,
    };
    let stdin_text = match stdin_value.tag {
        TeaValueTag::Nil => None,
        TeaValueTag::String => Some(unsafe {
            expect_string(
                stdin_value.payload.string_value,
                "process.run expects stdin to be a valid string",
            )
        }),
        _ => panic!(
            "{}",
            process_error("run", &command_str, "stdin must be a String")
        ),
    };

    let mut command_proc = Command::new(&command_str);
    if !arguments.is_empty() {
        command_proc.args(&arguments);
    }
    for (key, value) in &env_map {
        command_proc.env(key, value);
    }
    if let Some(dir) = &cwd_str {
        command_proc.current_dir(dir);
    }
    if stdin_text.is_some() {
        command_proc.stdin(Stdio::piped());
    } else {
        command_proc.stdin(Stdio::null());
    }
    command_proc.stdout(Stdio::piped());
    command_proc.stderr(Stdio::piped());

    let mut child = command_proc
        .spawn()
        .unwrap_or_else(|error| panic!("{}", process_error("run", &command_str, error)));

    if let Some(input) = stdin_text {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .unwrap_or_else(|error| panic!("{}", process_error("run", &command_str, error)));
        }
    }

    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("{}", process_error("run", &command_str, error)));
    let stdout_text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1) as i64;

    build_process_result_struct(
        template,
        exit_code,
        stdout_text,
        stderr_text,
        command_str.clone(),
    )
    .unwrap_or_else(|error| panic!("{}", process_error("run", &command_str, error)))
}

#[no_mangle]
pub extern "C" fn tea_process_spawn(
    command: *const TeaString,
    args: TeaValue,
    env: TeaValue,
    cwd: TeaValue,
) -> c_longlong {
    let command_str = expect_string(command, "process.spawn expects a valid command string");
    let arguments = tea_value_list_to_strings(args)
        .unwrap_or_else(|error| panic!("{}", process_error("spawn", &command_str, error)));
    let env_map = tea_value_dict_to_string_map(env)
        .unwrap_or_else(|error| panic!("{}", process_error("spawn", &command_str, error)));
    let cwd_str = match cwd.tag {
        TeaValueTag::Nil => None,
        TeaValueTag::String => Some(unsafe {
            expect_string(
                cwd.payload.string_value,
                "process.spawn expects cwd to be a valid string",
            )
        }),
        // Some clients accidentally pass other value kinds (e.g., Dict) when
        // targeting the LLVM backend. Treat those as if no cwd override was
        // supplied so we continue to run instead of aborting with a panic.
        _ => None,
    };

    let mut command_proc = Command::new(&command_str);
    if !arguments.is_empty() {
        command_proc.args(&arguments);
    }
    for (key, value) in &env_map {
        command_proc.env(key, value);
    }
    if let Some(dir) = &cwd_str {
        command_proc.current_dir(dir);
    }
    command_proc.stdin(Stdio::piped());
    command_proc.stdout(Stdio::piped());
    command_proc.stderr(Stdio::piped());

    let mut child = command_proc
        .spawn()
        .unwrap_or_else(|error| panic!("{}", process_error("spawn", &command_str, error)));
    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);
    let stdin = child.stdin.take();

    let mut table = process_handles().lock().unwrap();
    let handle_id = NEXT_PROCESS_HANDLE.fetch_add(1, Ordering::SeqCst);
    table.insert(
        handle_id,
        ProcessHandleEntry {
            child,
            stdout,
            stderr,
            stdin,
            command: command_str,
        },
    );
    handle_id as c_longlong
}

#[no_mangle]
pub extern "C" fn tea_process_read_stdout(handle: c_longlong, size: c_longlong) -> *mut TeaString {
    let mut table = process_handles().lock().unwrap();
    let target = format!("handle {}", handle);
    let entry = table.get_mut(&(handle as i64)).unwrap_or_else(|| {
        panic!(
            "{}",
            process_error("read_stdout", &target, "invalid process handle")
        )
    });
    let command = entry.command.clone();
    let limit = if size <= 0 { None } else { Some(size as usize) };
    let output = read_process_pipe(&mut entry.stdout, limit)
        .unwrap_or_else(|error| panic!("{}", process_error("read_stdout", &command, error)));
    alloc_tea_string(&output)
}

#[no_mangle]
pub extern "C" fn tea_process_read_stderr(handle: c_longlong, size: c_longlong) -> *mut TeaString {
    let mut table = process_handles().lock().unwrap();
    let target = format!("handle {}", handle);
    let entry = table.get_mut(&(handle as i64)).unwrap_or_else(|| {
        panic!(
            "{}",
            process_error("read_stderr", &target, "invalid process handle")
        )
    });
    let command = entry.command.clone();
    let limit = if size <= 0 { None } else { Some(size as usize) };
    let output = read_process_pipe(&mut entry.stderr, limit)
        .unwrap_or_else(|error| panic!("{}", process_error("read_stderr", &command, error)));
    alloc_tea_string(&output)
}

#[no_mangle]
pub extern "C" fn tea_process_write_stdin(handle: c_longlong, data: TeaValue) {
    let mut table = process_handles().lock().unwrap();
    let target = format!("handle {}", handle);
    let entry = table.get_mut(&(handle as i64)).unwrap_or_else(|| {
        panic!(
            "{}",
            process_error("write_stdin", &target, "invalid process handle")
        )
    });
    let command = entry.command.clone();
    let input = match data.tag {
        TeaValueTag::String => unsafe {
            expect_string(
                data.payload.string_value,
                "process.write_stdin expects a valid string",
            )
        },
        _ => panic!(
            "{}",
            process_error("write_stdin", &command, "stdin must be a String")
        ),
    };
    if let Some(stdin) = entry.stdin.as_mut() {
        stdin
            .write_all(input.as_bytes())
            .unwrap_or_else(|error| panic!("{}", process_error("write_stdin", &command, error)));
    } else {
        panic!(
            "{}",
            process_error("write_stdin", &command, "stdin has been closed")
        );
    }
}

#[no_mangle]
pub extern "C" fn tea_process_close_stdin(handle: c_longlong) {
    let mut table = process_handles().lock().unwrap();
    let target = format!("handle {}", handle);
    let entry = table.get_mut(&(handle as i64)).unwrap_or_else(|| {
        panic!(
            "{}",
            process_error("close_stdin", &target, "invalid process handle")
        )
    });
    entry.stdin.take();
}

#[no_mangle]
pub extern "C" fn tea_process_wait(
    template: *const TeaStructTemplate,
    handle: c_longlong,
) -> *mut TeaStructInstance {
    let mut table = process_handles().lock().unwrap();
    let target = format!("handle {}", handle);
    let mut entry = table.remove(&(handle as i64)).unwrap_or_else(|| {
        panic!(
            "{}",
            process_error("wait", &target, "invalid process handle")
        )
    });
    let command = entry.command.clone();
    let status = entry
        .child
        .wait()
        .unwrap_or_else(|error| panic!("{}", process_error("wait", &command, error)));
    let stdout_text = read_process_pipe(&mut entry.stdout, None)
        .unwrap_or_else(|error| panic!("{}", process_error("wait", &command, error)));
    let stderr_text = read_process_pipe(&mut entry.stderr, None)
        .unwrap_or_else(|error| panic!("{}", process_error("wait", &command, error)));
    entry.stdin.take();
    let exit_code = status.code().unwrap_or(-1) as i64;
    build_process_result_struct(
        template,
        exit_code,
        stdout_text,
        stderr_text,
        command.clone(),
    )
    .unwrap_or_else(|error| panic!("{}", process_error("wait", &command, error)))
}

#[no_mangle]
pub extern "C" fn tea_process_kill(handle: c_longlong) -> c_int {
    let mut table = process_handles().lock().unwrap();
    let target = format!("handle {}", handle);
    let entry = table.get_mut(&(handle as i64)).unwrap_or_else(|| {
        panic!(
            "{}",
            process_error("kill", &target, "invalid process handle")
        )
    });
    let command = entry.command.clone();
    entry
        .child
        .kill()
        .unwrap_or_else(|error| panic!("{}", process_error("kill", &command, error)));
    1
}

#[no_mangle]
pub extern "C" fn tea_process_close(handle: c_longlong) {
    let mut table = process_handles().lock().unwrap();
    if let Some(mut entry) = table.remove(&(handle as i64)) {
        let _ = entry.child.kill();
    }
}

#[no_mangle]
pub extern "C" fn tea_string_equal(left: *const TeaString, right: *const TeaString) -> c_int {
    unsafe {
        match (left.is_null(), right.is_null()) {
            (true, true) => return 1,
            (true, false) | (false, true) => return 0,
            (false, false) => {}
        }

        let left_ref = &*left;
        let right_ref = &*right;
        if left_ref.len != right_ref.len {
            return 0;
        }

        let left_bytes =
            std::slice::from_raw_parts(left_ref.data as *const u8, left_ref.len as usize);
        let right_bytes =
            std::slice::from_raw_parts(right_ref.data as *const u8, right_ref.len as usize);
        if left_bytes == right_bytes {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_list_equal(left: *const TeaList, right: *const TeaList) -> c_int {
    if left == right {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_int(value: TeaValue) -> c_longlong {
    unsafe {
        match value.tag {
            TeaValueTag::Int => value.payload.int_value,
            TeaValueTag::Nil => 0,
            _ => panic!("tea_value_as_int: value is not an Int"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_float(value: TeaValue) -> c_double {
    unsafe {
        match value.tag {
            TeaValueTag::Float => value.payload.float_value,
            TeaValueTag::Int => value.payload.int_value as c_double,
            _ => panic!("tea_value_as_float: value is not a Float"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_bool(value: TeaValue) -> c_int {
    unsafe {
        match value.tag {
            TeaValueTag::Bool => value.payload.bool_value,
            TeaValueTag::Nil => 0,
            _ => panic!("tea_value_as_bool: value is not a Bool"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_string(value: TeaValue) -> *const TeaString {
    unsafe {
        match value.tag {
            TeaValueTag::String => value.payload.string_value,
            _ => panic!("tea_value_as_string: value is not a String"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_list(value: TeaValue) -> *const TeaList {
    unsafe {
        match value.tag {
            TeaValueTag::List => value.payload.list_value,
            _ => panic!("tea_value_as_list: value is not a List"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_dict(value: TeaValue) -> *const TeaDict {
    unsafe {
        match value.tag {
            TeaValueTag::Dict => value.payload.dict_value,
            _ => panic!("tea_value_as_dict: value is not a Dict"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_struct(value: TeaValue) -> *const TeaStructInstance {
    unsafe {
        match value.tag {
            TeaValueTag::Struct => value.payload.struct_value,
            _ => panic!("tea_value_as_struct: value is not a Struct"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_as_closure(value: TeaValue) -> *const TeaClosure {
    unsafe {
        match value.tag {
            TeaValueTag::Closure => value.payload.closure_value,
            _ => panic!("tea_value_as_closure: value is not a Closure"),
        }
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_int(value: c_longlong) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Int,
        payload: TeaValuePayload { int_value: value },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_float(value: c_double) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Float,
        payload: TeaValuePayload { float_value: value },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_bool(value: c_int) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Bool,
        payload: TeaValuePayload { bool_value: value },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_string(value: *const TeaString) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::String,
        payload: TeaValuePayload {
            string_value: value,
        },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_list(value: *const TeaList) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::List,
        payload: TeaValuePayload { list_value: value },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_dict(value: *const TeaDict) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Dict,
        payload: TeaValuePayload { dict_value: value },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_struct(value: *const TeaStructInstance) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Struct,
        payload: TeaValuePayload {
            struct_value: value,
        },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_from_closure(value: *const TeaClosure) -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Closure,
        payload: TeaValuePayload {
            closure_value: value,
        },
    }
}

#[no_mangle]
pub extern "C" fn tea_value_nil() -> TeaValue {
    TeaValue {
        tag: TeaValueTag::Nil,
        payload: TeaValuePayload { int_value: 0 },
    }
}
