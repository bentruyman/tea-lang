use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use std::rc::Rc;
use std::time::UNIX_EPOCH;

use path_clean::PathClean;
use pathdiff::diff_paths;
use tea_support::fs_error;
use tempfile::NamedTempFile;

use super::bytecode::{Instruction, Program, TypeCheck};
use super::cli::{CliParseOutcome, CliScopeOutcome};
use super::value::{
    ClosureInstance, ErrorTemplate, ErrorVariantValue, StructInstance, StructTemplate, Value,
};
use crate::ast::SourceSpan;
use crate::stdlib::StdFunctionKind;
use serde_json::Value as JsonValue;

#[derive(Debug)]
pub enum VmError {
    Runtime(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::Runtime(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for VmError {}

#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub name: String,
    pub span: SourceSpan,
    pub status: TestStatus,
}

#[derive(Debug, Clone)]
pub enum TestStatus {
    Passed,
    Failed { message: String },
}

#[derive(Clone, Debug)]
pub struct TestRunOptions {
    pub update_snapshots: bool,
    pub snapshot_root: PathBuf,
    pub relative_test_path: PathBuf,
}

#[derive(Clone, Debug)]
struct SnapshotSettings {
    update: bool,
    snapshot_root: PathBuf,
    relative_test_path: PathBuf,
}

impl From<&TestRunOptions> for SnapshotSettings {
    fn from(options: &TestRunOptions) -> Self {
        Self {
            update: options.update_snapshots,
            snapshot_root: options.snapshot_root.clone(),
            relative_test_path: options.relative_test_path.clone(),
        }
    }
}

pub struct Vm {
    program: Program,
    stack: Vec<Value>,
    globals: Vec<Value>,
    frames: Vec<Frame>,
    catch_stack: Vec<CatchFrame>,
    #[allow(dead_code)]
    next_fs_handle: i64,
    #[allow(dead_code)]
    fs_handles: HashMap<i64, FsReadHandle>,
    #[allow(dead_code)]
    next_process_handle: i64,
    #[allow(dead_code)]
    process_handles: HashMap<i64, ProcessEntry>,
    snapshot_settings: Option<SnapshotSettings>,
    #[allow(dead_code)]
    cli_result_template: Rc<StructTemplate>,
    #[allow(dead_code)]
    cli_parse_result_template: Rc<StructTemplate>,
    #[allow(dead_code)]
    process_result_template: Rc<StructTemplate>,
    error_templates: Vec<Rc<ErrorTemplate>>,
    cli_args: Vec<String>,
    program_name: Option<String>,
}

#[derive(Clone)]
struct Frame {
    chunk: ChunkRef,
    ip: usize,
    stack_start: usize,
}

#[derive(Clone)]
enum ChunkRef {
    Main,
    Function(usize),
}

// Structs for removed fs/process functionality - kept for potential future use
#[allow(dead_code)]
struct FsReadHandle {
    reader: BufReader<File>,
}

#[allow(dead_code)]
struct ProcessEntry {
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
    stderr: Option<BufReader<ChildStderr>>,
    stdin: Option<ChildStdin>,
    command: String,
}

#[derive(Clone)]
struct CatchFrame {
    frame_index: usize,
    stack_len: usize,
    chunk: ChunkRef,
    handler_ip: usize,
}

// Helper functions for removed fs functionality - kept for potential future use
#[allow(dead_code)]
fn vm_write_atomic(path: &Path, data: &[u8]) -> Result<(), VmError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let target = path.display().to_string();
    let mut temp = NamedTempFile::new_in(parent)
        .map_err(|error| VmError::Runtime(fs_error("write_atomic", &target, &error)))?;
    temp.write_all(data)
        .map_err(|error| VmError::Runtime(fs_error("write_atomic", &target, &error)))?;
    temp.flush()
        .map_err(|error| VmError::Runtime(fs_error("write_atomic", &target, &error)))?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|error| VmError::Runtime(fs_error("write_atomic", &target, &error.error)))?;
    Ok(())
}

fn vm_strings_to_list(entries: Vec<String>) -> Value {
    let values = entries.into_iter().map(Value::String).collect::<Vec<_>>();
    Value::List(Rc::new(values))
}

#[allow(dead_code)]
fn vm_collect_modified(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
}

#[allow(dead_code)]
#[cfg(unix)]
fn vm_metadata_mode(metadata: &fs::Metadata) -> i64 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() as i64
}

#[allow(dead_code)]
#[cfg(windows)]
fn vm_metadata_mode(metadata: &fs::Metadata) -> i64 {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() as i64
}

#[allow(dead_code)]
#[cfg(not(any(unix, windows)))]
fn vm_metadata_mode(metadata: &fs::Metadata) -> i64 {
    let _ = metadata;
    0
}

#[allow(dead_code)]
fn vm_metadata_value(path: &Path, metadata: fs::Metadata) -> Value {
    let mut map = HashMap::new();
    map.insert(
        "path".to_string(),
        Value::String(path.to_string_lossy().into_owned()),
    );
    map.insert("is_dir".to_string(), Value::Bool(metadata.is_dir()));
    map.insert("is_file".to_string(), Value::Bool(metadata.is_file()));
    map.insert(
        "is_symlink".to_string(),
        Value::Bool(metadata.file_type().is_symlink()),
    );
    map.insert(
        "readonly".to_string(),
        Value::Bool(metadata.permissions().readonly()),
    );
    map.insert("size".to_string(), Value::Int(metadata.len() as i64));
    map.insert(
        "permissions".to_string(),
        Value::Int(vm_metadata_mode(&metadata)),
    );
    match vm_collect_modified(&metadata) {
        Some(timestamp) => {
            map.insert("modified".to_string(), Value::Int(timestamp));
        }
        None => {
            map.insert("modified".to_string(), Value::Nil);
        }
    }
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            map.insert(
                "parent".to_string(),
                Value::String(parent.to_string_lossy().into_owned()),
            );
        }
        _ => {
            map.insert("parent".to_string(), Value::Nil);
        }
    }
    Value::Dict(Rc::new(map))
}

impl Vm {
    pub fn new(program: &Program) -> Self {
        let globals = vec![Value::Nil; program.globals.len()];
        let cli_template = Rc::new(StructTemplate {
            name: "CliResult".to_string(),
            field_names: vec![
                "exit".to_string(),
                "stdout".to_string(),
                "stderr".to_string(),
            ],
        });
        let cli_parse_template = Rc::new(StructTemplate {
            name: "CliParseResult".to_string(),
            field_names: vec![
                "ok".to_string(),
                "exit".to_string(),
                "command".to_string(),
                "path".to_string(),
                "options".to_string(),
                "positionals".to_string(),
                "scopes".to_string(),
                "rest".to_string(),
                "message".to_string(),
                "help".to_string(),
            ],
        });
        let process_template = Rc::new(StructTemplate {
            name: "ProcessResult".to_string(),
            field_names: vec![
                "exit".to_string(),
                "success".to_string(),
                "stdout".to_string(),
                "stderr".to_string(),
                "command".to_string(),
            ],
        });

        Self {
            program: program.clone(),
            stack: Vec::new(),
            globals,
            frames: vec![Frame {
                chunk: ChunkRef::Main,
                ip: 0,
                stack_start: 0,
            }],
            catch_stack: Vec::new(),
            next_fs_handle: 1,
            fs_handles: HashMap::new(),
            next_process_handle: 1,
            process_handles: HashMap::new(),
            snapshot_settings: None,
            cli_result_template: cli_template,
            cli_parse_result_template: cli_parse_template,
            process_result_template: process_template,
            error_templates: program.errors.clone(),
            cli_args: Vec::new(),
            program_name: None,
        }
    }

    pub fn set_cli_context<S: Into<String>>(&mut self, program_name: S, args: Vec<String>) {
        self.program_name = Some(program_name.into());
        self.cli_args = args;
    }

    pub fn set_cli_args(&mut self, args: Vec<String>) {
        self.cli_args = args;
    }

    pub fn set_program_name<S: Into<String>>(&mut self, program_name: S) {
        self.program_name = Some(program_name.into());
    }

    pub fn run(&mut self) -> Result<Value> {
        loop {
            if self.frames.is_empty() {
                return Ok(Value::Nil);
            }

            let frame_index = self.frames.len() - 1;
            let chunk_ref = self.frames[frame_index].chunk.clone();

            let instruction = {
                let chunk = self.resolve_chunk(&chunk_ref);
                if self.frames[frame_index].ip >= chunk.instructions.len() {
                    bail!(VmError::Runtime(
                        "instruction pointer out of bounds".to_string()
                    ));
                }
                chunk.instructions[self.frames[frame_index].ip].clone()
            };
            self.frames[frame_index].ip += 1;

            match instruction {
                Instruction::Constant(index) => {
                    let chunk = self.resolve_chunk(&chunk_ref);
                    let value = chunk.constants.get(index).cloned().ok_or_else(|| {
                        VmError::Runtime(format!("invalid constant index {index}"))
                    })?;
                    self.stack.push(value);
                }
                Instruction::GetGlobal(index) => {
                    let value =
                        self.globals.get(index).cloned().ok_or_else(|| {
                            VmError::Runtime(format!("invalid global index {index}"))
                        })?;
                    self.stack.push(value);
                }
                Instruction::SetGlobal(index) => {
                    let value = self.pop()?;
                    if let Some(slot) = self.globals.get_mut(index) {
                        *slot = value.clone();
                    } else {
                        bail!(VmError::Runtime(format!("invalid global index {index}")));
                    }
                    self.stack.push(value);
                }
                Instruction::Pop => {
                    self.pop()?;
                }
                Instruction::GetLocal(index) => {
                    let slot = self.local_index(frame_index, index)?;
                    let value =
                        self.stack.get(slot).cloned().ok_or_else(|| {
                            VmError::Runtime(format!("invalid local index {index}"))
                        })?;
                    self.stack.push(value);
                }
                Instruction::SetLocal(index) => {
                    let value = self.pop()?;
                    let slot = self.local_index(frame_index, index)?;
                    if slot >= self.stack.len() {
                        self.stack.resize(slot + 1, Value::Nil);
                    }
                    if let Some(existing) = self.stack.get_mut(slot) {
                        *existing = value.clone();
                    }
                    self.stack.push(value);
                }
                Instruction::Add => {
                    let right = self.pop()?;
                    let left = self.pop()?;

                    // Handle string concatenation
                    if let (Value::String(a), Value::String(b)) = (&left, &right) {
                        let mut result = String::with_capacity(a.len() + b.len());
                        result.push_str(a);
                        result.push_str(b);
                        self.stack.push(Value::String(result));
                    } else if let (Value::List(a), Value::List(b)) = (&left, &right) {
                        // Handle list concatenation
                        let mut result = Vec::with_capacity(a.len() + b.len());
                        result.extend(a.as_ref().iter().cloned());
                        result.extend(b.as_ref().iter().cloned());
                        self.stack.push(Value::List(Rc::new(result)));
                    } else {
                        // Handle numeric addition
                        let result = match (left, right) {
                            (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                            (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                            (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 + b),
                            (Value::Float(a), Value::Int(b)) => Value::Float(a + b as f64),
                            _ => {
                                return Err(VmError::Runtime(
                                    "addition requires numeric, string, or list operands"
                                        .to_string(),
                                )
                                .into());
                            }
                        };
                        self.stack.push(result);
                    }
                }
                Instruction::Subtract => self.perform_numeric_binary(
                    |a, b| Ok(Value::Int(a - b)),
                    |a, b| Ok(Value::Float(a - b)),
                )?,
                Instruction::Multiply => self.perform_numeric_binary(
                    |a, b| Ok(Value::Int(a * b)),
                    |a, b| Ok(Value::Float(a * b)),
                )?,
                Instruction::Divide => self.perform_numeric_binary(
                    |a, b| {
                        if b == 0 {
                            Err(VmError::Runtime("division by zero".to_string()).into())
                        } else {
                            Ok(Value::Int(a / b))
                        }
                    },
                    |a, b| {
                        if b == 0.0 {
                            Err(VmError::Runtime("division by zero".to_string()).into())
                        } else {
                            Ok(Value::Float(a / b))
                        }
                    },
                )?,
                Instruction::Modulo => self.perform_numeric_binary(
                    |a, b| {
                        if b == 0 {
                            Err(VmError::Runtime("modulo by zero".to_string()).into())
                        } else {
                            Ok(Value::Int(a % b))
                        }
                    },
                    |a, b| {
                        if b == 0.0 {
                            Err(VmError::Runtime("modulo by zero".to_string()).into())
                        } else {
                            Ok(Value::Float(a % b))
                        }
                    },
                )?,
                Instruction::Equal => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(a == b));
                }
                Instruction::NotEqual => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(a != b));
                }
                Instruction::Greater => {
                    self.perform_numeric_comparison(|a, b| a > b, |a, b| a > b)?
                }
                Instruction::GreaterEqual => {
                    self.perform_numeric_comparison(|a, b| a >= b, |a, b| a >= b)?
                }
                Instruction::Less => self.perform_numeric_comparison(|a, b| a < b, |a, b| a < b)?,
                Instruction::LessEqual => {
                    self.perform_numeric_comparison(|a, b| a <= b, |a, b| a <= b)?
                }
                Instruction::Negate => {
                    let value = self.pop()?;
                    match value {
                        Value::Int(v) => self.stack.push(Value::Int(-v)),
                        Value::Float(v) => self.stack.push(Value::Float(-v)),
                        _ => bail!(VmError::Runtime(
                            "negation expects a numeric operand".to_string()
                        )),
                    }
                }
                Instruction::Not => {
                    let value = self.pop()?;
                    self.stack.push(Value::Bool(!value.is_truthy()));
                }
                Instruction::TypeIs(type_check) => {
                    let value = self.pop()?;
                    let result = self.value_matches_type(&value, &type_check);
                    self.stack.push(Value::Bool(result));
                }
                Instruction::Jump(target) => {
                    self.frames[frame_index].ip = target;
                }
                Instruction::JumpIfFalse(target) => {
                    let value = self.pop()?;
                    if !value.is_truthy() {
                        self.frames[frame_index].ip = target;
                    }
                }
                Instruction::JumpIfNil(target) => {
                    if matches!(self.stack.last(), Some(Value::Nil)) {
                        self.frames[frame_index].ip = target;
                    }
                }
                Instruction::Print => {
                    let value = self.pop()?;
                    println!("{value}");
                    self.stack.push(Value::Void);
                }
                Instruction::BuiltinCall { kind, arg_count } => {
                    self.execute_builtin(kind, arg_count)?;
                }
                Instruction::Call(arg_count) => {
                    self.call_function(arg_count)?;
                }
                Instruction::MakeList(count) => {
                    if self.stack.len() < count {
                        bail!(VmError::Runtime(
                            "not enough values to build list".to_string()
                        ));
                    }
                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.pop()?);
                    }
                    elements.reverse();
                    self.stack.push(Value::List(Rc::new(elements)));
                }
                Instruction::Index => {
                    let index_value = self.pop()?;
                    let collection_value = self.pop()?;
                    match (collection_value, index_value) {
                        (Value::List(list), Value::Int(index)) => {
                            if index < 0 {
                                bail!(VmError::Runtime("negative index".to_string()));
                            }
                            let idx = index as usize;
                            if idx >= list.len() {
                                bail!(VmError::Runtime("index out of bounds".to_string()));
                            }
                            self.stack.push(list[idx].clone());
                        }
                        (Value::Dict(map), Value::String(key)) => {
                            if let Some(value) = map.get(&key) {
                                self.stack.push(value.clone());
                            } else {
                                bail!(VmError::Runtime(format!(
                                    "missing key '{}' in dictionary",
                                    key
                                )));
                            }
                        }
                        (Value::String(s), Value::Int(index)) => {
                            if index < 0 {
                                bail!(VmError::Runtime("negative index".to_string()));
                            }
                            let chars: Vec<char> = s.chars().collect();
                            let idx = index as usize;
                            if idx >= chars.len() {
                                bail!(VmError::Runtime("index out of bounds".to_string()));
                            }
                            self.stack.push(Value::String(chars[idx].to_string()));
                        }
                        (Value::List(_), _) => {
                            bail!(VmError::Runtime("list index must be an Int".to_string()));
                        }
                        (Value::Dict(_), _) => {
                            bail!(VmError::Runtime(
                                "dictionary index must be a String".to_string()
                            ));
                        }
                        (Value::String(_), _) => {
                            bail!(VmError::Runtime("string index must be an Int".to_string()));
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "indexing requires a list, dictionary, or string value".to_string()
                            ));
                        }
                    }
                }
                Instruction::Slice { inclusive } => {
                    let end_value = self.pop()?;
                    let start_value = self.pop()?;
                    let collection_value = self.pop()?;

                    let (Value::Int(start), Value::Int(end)) = (start_value, end_value) else {
                        bail!(VmError::Runtime(
                            "slice indices must be integers".to_string()
                        ));
                    };

                    if start < 0 || end < 0 {
                        bail!(VmError::Runtime(
                            "slice indices cannot be negative".to_string()
                        ));
                    }

                    let start_idx = start as usize;
                    let mut end_idx = end as usize;

                    if inclusive {
                        end_idx = end_idx.saturating_add(1);
                    }

                    match collection_value {
                        Value::String(s) => {
                            let chars: Vec<char> = s.chars().collect();
                            if start_idx > chars.len() {
                                bail!(VmError::Runtime(
                                    "slice start index out of bounds".to_string()
                                ));
                            }
                            let end_idx = end_idx.min(chars.len());
                            if start_idx > end_idx {
                                bail!(VmError::Runtime("slice start must be <= end".to_string()));
                            }
                            let slice: String = chars[start_idx..end_idx].iter().collect();
                            self.stack.push(Value::String(slice));
                        }
                        Value::List(list) => {
                            if start_idx > list.len() {
                                bail!(VmError::Runtime(
                                    "slice start index out of bounds".to_string()
                                ));
                            }
                            let end_idx = end_idx.min(list.len());
                            if start_idx > end_idx {
                                bail!(VmError::Runtime("slice start must be <= end".to_string()));
                            }
                            let slice: Vec<Value> = list[start_idx..end_idx].to_vec();
                            self.stack.push(Value::List(Rc::new(slice)));
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "slicing requires a list or string".to_string()
                            ));
                        }
                    }
                }
                Instruction::SetIndex => {
                    let new_value = self.pop()?;
                    let index_value = self.pop()?;
                    let collection = self.pop()?;
                    match (collection, index_value) {
                        (Value::List(list), Value::Int(index)) => {
                            if index < 0 {
                                bail!(VmError::Runtime("negative index".to_string()));
                            }
                            let idx = index as usize;
                            let mut new_list = (*list).clone();
                            if idx >= new_list.len() {
                                bail!(VmError::Runtime("index out of bounds".to_string()));
                            }
                            new_list[idx] = new_value;
                            self.stack.push(Value::List(Rc::new(new_list)));
                        }
                        (Value::Dict(map), Value::String(key)) => {
                            let mut new_map = (*map).clone();
                            new_map.insert(key, new_value);
                            self.stack.push(Value::Dict(Rc::new(new_map)));
                        }
                        (Value::List(_), _) => {
                            bail!(VmError::Runtime("list index must be an Int".to_string()));
                        }
                        (Value::Dict(_), _) => {
                            bail!(VmError::Runtime(
                                "dictionary index must be a String".to_string()
                            ));
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "indexed assignment requires a list or dictionary".to_string()
                            ));
                        }
                    }
                }
                Instruction::MakeDict(count) => {
                    if self.stack.len() < count * 2 {
                        bail!(VmError::Runtime(
                            "not enough values to build dict".to_string()
                        ));
                    }
                    let mut entries = HashMap::with_capacity(count);
                    for _ in 0..count {
                        let value = self.pop()?;
                        let key = self.pop()?;
                        match key {
                            Value::String(key) => {
                                entries.insert(key, value);
                            }
                            _ => {
                                bail!(VmError::Runtime(
                                    "dictionary keys must be strings".to_string()
                                ));
                            }
                        }
                    }
                    self.stack.push(Value::Dict(Rc::new(entries)));
                }
                Instruction::DictKeys => {
                    let dict_value = self.pop()?;
                    match dict_value {
                        Value::Dict(map) => {
                            // Get keys in insertion order (HashMap iteration is deterministic in Rust)
                            let keys: Vec<Value> =
                                map.keys().map(|k| Value::String(k.clone())).collect();
                            self.stack.push(Value::List(Rc::new(keys)));
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "DictKeys instruction requires a dict value".to_string()
                            ));
                        }
                    }
                }
                Instruction::GetField => {
                    let field_value = self.pop()?;
                    let object_value = self.pop()?;
                    match (object_value, field_value) {
                        (Value::Dict(map), Value::String(field)) => {
                            if let Some(value) = map.get(&field) {
                                self.stack.push(value.clone());
                            } else {
                                bail!(VmError::Runtime(format!(
                                    "missing key '{}' in dictionary",
                                    field
                                )));
                            }
                        }
                        (Value::Struct(instance), Value::String(field)) => {
                            if let Some(index) = instance
                                .template
                                .field_names
                                .iter()
                                .position(|name| name == &field)
                            {
                                if let Some(value) = instance.fields.get(index) {
                                    self.stack.push(value.clone());
                                } else {
                                    bail!(VmError::Runtime(format!(
                                        "struct '{}' missing field '{}'",
                                        instance.template.name, field
                                    )));
                                }
                            } else {
                                bail!(VmError::Runtime(format!(
                                    "struct '{}' has no field '{}'",
                                    instance.template.name, field
                                )));
                            }
                        }
                        (Value::Error(error), Value::String(field)) => {
                            let template = self
                                .error_templates
                                .iter()
                                .find(|template| {
                                    template.error_name == error.error_name
                                        && template.variant_name == error.variant_name
                                })
                                .cloned()
                                .ok_or_else(|| {
                                    VmError::Runtime(format!(
                                        "missing template for error '{}.{}'",
                                        error.error_name, error.variant_name
                                    ))
                                })?;
                            if let Some(index) =
                                template.field_names.iter().position(|name| name == &field)
                            {
                                if let Some(value) = error.fields.get(index) {
                                    self.stack.push(value.clone());
                                } else {
                                    bail!(VmError::Runtime(format!(
                                        "error '{}.{}' missing field value '{}'",
                                        template.error_name, template.variant_name, field
                                    )));
                                }
                            } else {
                                bail!(VmError::Runtime(format!(
                                    "error '{}.{}' has no field '{}'",
                                    template.error_name, template.variant_name, field
                                )));
                            }
                        }
                        (Value::Dict(_), _) => {
                            bail!(VmError::Runtime(
                                "dictionary field access requires string key".to_string()
                            ));
                        }
                        (Value::Struct(instance), key) => {
                            bail!(VmError::Runtime(format!(
                                "struct '{}' field access expects string key, found {:?}",
                                instance.template.name, key
                            )));
                        }
                        (other, key) => {
                            bail!(VmError::Runtime(format!(
                                "unsupported field access on value {:?} with key {:?}",
                                other, key
                            )));
                        }
                    }
                }
                Instruction::MakeStructPositional(struct_index) => {
                    let template = self
                        .program
                        .structs
                        .get(struct_index)
                        .ok_or_else(|| {
                            VmError::Runtime(format!("unknown struct index {}", struct_index))
                        })?
                        .clone();
                    let field_count = template.field_names.len();
                    if self.stack.len() < field_count {
                        bail!(VmError::Runtime(
                            "not enough values to build struct".to_string()
                        ));
                    }
                    let mut values = Vec::with_capacity(field_count);
                    for _ in 0..field_count {
                        values.push(self.pop()?);
                    }
                    values.reverse();
                    let instance = StructInstance {
                        template,
                        fields: values,
                    };
                    self.stack.push(Value::Struct(Rc::new(instance)));
                }
                Instruction::MakeStructNamed(struct_index) => {
                    let template = self
                        .program
                        .structs
                        .get(struct_index)
                        .ok_or_else(|| {
                            VmError::Runtime(format!("unknown struct index {}", struct_index))
                        })?
                        .clone();
                    let field_count = template.field_names.len();
                    if self.stack.len() < field_count * 2 {
                        bail!(VmError::Runtime(
                            "not enough values to build struct".to_string()
                        ));
                    }
                    let mut slots: Vec<Option<Value>> = vec![None; field_count];
                    let mut seen = HashSet::new();
                    for _ in 0..field_count {
                        let value = self.pop()?;
                        let key = self.pop()?;
                        let field_name = match key {
                            Value::String(name) => name,
                            other => {
                                bail!(VmError::Runtime(format!(
                                    "struct field name must be String, found {:?}",
                                    other
                                )));
                            }
                        };
                        if !seen.insert(field_name.clone()) {
                            bail!(VmError::Runtime(format!(
                                "duplicate field '{}' while constructing struct '{}'",
                                field_name, template.name
                            )));
                        }
                        if let Some(index) = template
                            .field_names
                            .iter()
                            .position(|name| name == &field_name)
                        {
                            if slots[index].is_some() {
                                bail!(VmError::Runtime(format!(
                                    "duplicate field '{}' while constructing struct '{}'",
                                    field_name, template.name
                                )));
                            }
                            slots[index] = Some(value);
                        } else {
                            bail!(VmError::Runtime(format!(
                                "struct '{}' has no field '{}'",
                                template.name, field_name
                            )));
                        }
                    }

                    let mut values = Vec::with_capacity(field_count);
                    for (index, entry) in slots.into_iter().enumerate() {
                        match entry {
                            Some(value) => values.push(value),
                            None => {
                                let field_name = &template.field_names[index];
                                bail!(VmError::Runtime(format!(
                                    "missing value for field '{}' in struct '{}'",
                                    field_name, template.name
                                )));
                            }
                        }
                    }

                    let instance = StructInstance {
                        template,
                        fields: values,
                    };
                    self.stack.push(Value::Struct(Rc::new(instance)));
                }
                Instruction::MakeError {
                    error_index,
                    field_count,
                } => {
                    if self.stack.len() < field_count {
                        bail!(VmError::Runtime(
                            "not enough values to build error".to_string()
                        ));
                    }
                    let mut fields = Vec::with_capacity(field_count);
                    for _ in 0..field_count {
                        fields.push(self.pop()?);
                    }
                    fields.reverse();
                    let template =
                        self.error_templates
                            .get(error_index)
                            .cloned()
                            .ok_or_else(|| {
                                VmError::Runtime(format!("unknown error index {}", error_index))
                            })?;
                    let error_value = ErrorVariantValue {
                        error_name: template.error_name.clone(),
                        variant_name: template.variant_name.clone(),
                        fields,
                    };
                    self.stack.push(Value::Error(Rc::new(error_value)));
                }
                Instruction::PushCatch { handler_ip } => {
                    self.catch_stack.push(CatchFrame {
                        frame_index,
                        stack_len: self.stack.len(),
                        chunk: chunk_ref.clone(),
                        handler_ip,
                    });
                }
                Instruction::PopCatch => {
                    if self.catch_stack.pop().is_none() {
                        bail!(VmError::Runtime(
                            "attempted to pop catch frame but none are active".to_string()
                        ));
                    }
                }
                Instruction::Throw => {
                    let value = self.pop()?;
                    let error = match value {
                        Value::Error(error) => error,
                        other => {
                            bail!(VmError::Runtime(format!(
                                "throw expects an error value, found {:?}",
                                other
                            )))
                        }
                    };

                    let catch_frame = loop {
                        match self.catch_stack.pop() {
                            Some(frame) if frame.frame_index < self.frames.len() => break frame,
                            Some(_) => continue,
                            None => {
                                let message =
                                    format!("uncaught error {}", Value::Error(error.clone()));
                                return Err(VmError::Runtime(message).into());
                            }
                        }
                    };

                    while self.frames.len() - 1 > catch_frame.frame_index {
                        let frame = self.frames.pop().ok_or_else(|| {
                            VmError::Runtime("call frame stack underflow during throw".to_string())
                        })?;
                        self.stack.truncate(frame.stack_start);
                    }

                    if let Some(frame) = self.frames.get_mut(catch_frame.frame_index) {
                        frame.chunk = catch_frame.chunk.clone();
                        frame.ip = catch_frame.handler_ip;
                    } else {
                        bail!(VmError::Runtime(
                            "missing call frame for catch handler".to_string()
                        ));
                    }

                    if self.stack.len() < catch_frame.stack_len {
                        bail!(VmError::Runtime(
                            "stack underflow while unwinding throw".to_string()
                        ));
                    }
                    self.stack.truncate(catch_frame.stack_len);
                    self.stack.push(Value::Error(error));
                }
                Instruction::MakeClosure {
                    function_index,
                    capture_count,
                } => {
                    if self.stack.len() < capture_count {
                        bail!(VmError::Runtime(
                            "not enough values to build closure".to_string()
                        ));
                    }
                    let mut captures = Vec::with_capacity(capture_count);
                    for _ in 0..capture_count {
                        captures.push(self.pop()?);
                    }
                    captures.reverse();
                    let captures = Rc::new(captures);
                    let closure = ClosureInstance {
                        function_index,
                        captures,
                    };
                    self.stack.push(Value::Closure(Rc::new(closure)));
                }
                Instruction::ConcatStrings(count) => {
                    let mut parts = Vec::with_capacity(count);
                    for _ in 0..count {
                        let value = self.pop()?;
                        let text = match value {
                            Value::String(text) => text,
                            other => other.to_string(),
                        };
                        parts.push(text);
                    }
                    parts.reverse();
                    let mut result = String::new();
                    for part in parts {
                        result.push_str(&part);
                    }
                    self.stack.push(Value::String(result));
                }
                Instruction::AssertNonNil => {
                    if matches!(self.stack.last(), Some(Value::Nil)) {
                        bail!(VmError::Runtime(
                            "attempted to unwrap a nil value at runtime".to_string()
                        ));
                    }
                }
                Instruction::Return => {
                    let value = self.pop().unwrap_or(Value::Void);
                    let frame = self.frames.pop().unwrap();
                    self.stack.truncate(frame.stack_start);
                    self.stack.push(value.clone());

                    if self.frames.is_empty() {
                        return Ok(value);
                    }
                }
            }
        }
    }

    pub fn run_tests(
        &mut self,
        filter: Option<&str>,
        options: Option<&TestRunOptions>,
    ) -> Result<Vec<TestOutcome>> {
        if let Some(opts) = options {
            self.snapshot_settings = Some(SnapshotSettings::from(opts));
        } else {
            self.snapshot_settings = None;
        }

        if self
            .frames
            .iter()
            .any(|frame| matches!(frame.chunk, ChunkRef::Main))
        {
            let _ = self.run()?;
            if !self.stack.is_empty() {
                self.stack.pop();
            }
        }

        let mut outcomes = Vec::new();
        let tests = self.program.tests.clone();
        let filter = filter.map(|f| f.to_ascii_lowercase());

        for test in tests.into_iter().filter(|test| {
            if let Some(ref needle) = filter {
                test.name.to_ascii_lowercase().contains(needle)
            } else {
                true
            }
        }) {
            let stack_checkpoint = self.stack.len();
            let frame_checkpoint = self.frames.len();

            self.stack.push(Value::Function(test.function_index));
            self.frames.push(Frame {
                chunk: ChunkRef::Function(test.function_index),
                ip: 0,
                stack_start: stack_checkpoint,
            });

            let status = match self.run() {
                Ok(_) => TestStatus::Passed,
                Err(error) => {
                    self.stack.truncate(stack_checkpoint);
                    self.frames.truncate(frame_checkpoint);
                    TestStatus::Failed {
                        message: error.to_string(),
                    }
                }
            };

            if self.stack.len() > stack_checkpoint {
                self.stack.truncate(stack_checkpoint);
            }

            outcomes.push(TestOutcome {
                name: test.name,
                span: test.name_span,
                status,
            });
        }

        self.snapshot_settings = None;
        Ok(outcomes)
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack.pop().ok_or_else(|| {
            anyhow::Error::new(VmError::Runtime(
                "attempted to pop from empty stack".to_string(),
            ))
        })
    }

    fn pop_n(&mut self, count: usize) -> Result<Vec<Value>> {
        if self.stack.len() < count {
            bail!(VmError::Runtime(
                "not enough values on stack for builtin call".to_string(),
            ));
        }
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.pop()?);
        }
        values.reverse();
        Ok(values)
    }

    fn execute_builtin(&mut self, kind: StdFunctionKind, arg_count: usize) -> Result<()> {
        let args = self.pop_n(arg_count)?;

        // Try to dispatch to an intrinsic first
        if let Some(intrinsic) = crate::runtime::get_intrinsic(kind) {
            let result = (intrinsic.impl_fn)(self, args)?;
            self.stack.push(result);
            return Ok(());
        }

        // Handle non-intrinsic builtins
        match kind {
            StdFunctionKind::Print => {
                bail!(VmError::Runtime(
                    "print builtin should not use BuiltinCall instruction".to_string(),
                ))
            }
            StdFunctionKind::Length => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "length expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let length = match &args[0] {
                    Value::String(text) => text.chars().count() as i64,
                    Value::List(items) => items.len() as i64,
                    Value::Dict(map) => map.len() as i64,
                    _ => {
                        bail!(VmError::Runtime(
                            "length expects a String, List, or Dict".to_string()
                        ))
                    }
                };
                self.stack.push(Value::Int(length));
            }
            StdFunctionKind::Exit => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "exit expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let code = match args[0] {
                    Value::Int(n) => n as i32,
                    _ => {
                        bail!(VmError::Runtime("exit expects an Int".to_string()))
                    }
                };
                std::process::exit(code);
            }
            StdFunctionKind::Delete => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "delete expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let dict = match &args[0] {
                    Value::Dict(map) => map,
                    _ => {
                        bail!(VmError::Runtime(
                            "delete expects a Dict as first argument".to_string()
                        ))
                    }
                };
                let key = match &args[1] {
                    Value::String(k) => k.as_str(),
                    _ => {
                        bail!(VmError::Runtime(
                            "delete expects a String as second argument".to_string()
                        ))
                    }
                };
                let mut new_dict = (**dict).clone();
                new_dict.remove(key);
                self.stack.push(Value::Dict(std::rc::Rc::new(new_dict)));
            }
            StdFunctionKind::Clear => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "clear expected 1 argument but got {}",
                        args.len()
                    )));
                }
                match args[0] {
                    Value::Dict(_) => {
                        self.stack.push(Value::Dict(std::rc::Rc::new(
                            std::collections::HashMap::new(),
                        )));
                    }
                    _ => {
                        bail!(VmError::Runtime("clear expects a Dict".to_string()))
                    }
                }
            }
            StdFunctionKind::Max => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "max expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let result = match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Value::Int(*a.max(b)),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a.max(*b)),
                    (Value::Int(a), Value::Float(b)) => Value::Float((*a as f64).max(*b)),
                    (Value::Float(a), Value::Int(b)) => Value::Float(a.max(*b as f64)),
                    _ => {
                        bail!(VmError::Runtime(
                            "max expects two numbers (Int or Float)".to_string()
                        ))
                    }
                };
                self.stack.push(result);
            }
            StdFunctionKind::Min => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "min expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let result = match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Value::Int(*a.min(b)),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a.min(*b)),
                    (Value::Int(a), Value::Float(b)) => Value::Float((*a as f64).min(*b)),
                    (Value::Float(a), Value::Int(b)) => Value::Float(a.min(*b as f64)),
                    _ => {
                        bail!(VmError::Runtime(
                            "min expects two numbers (Int or Float)".to_string()
                        ))
                    }
                };
                self.stack.push(result);
            }
            StdFunctionKind::Append => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "append expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let list = match &args[0] {
                    Value::List(items) => items,
                    _ => {
                        bail!(VmError::Runtime(
                            "append expects a List as first argument".to_string()
                        ))
                    }
                };
                let mut new_list = (**list).clone();
                new_list.push(args[1].clone());
                self.stack.push(Value::List(std::rc::Rc::new(new_list)));
            }
            StdFunctionKind::Assert => {
                if !(1..=2).contains(&args.len()) {
                    bail!(VmError::Runtime(format!(
                        "assert expected 1 or 2 arguments but got {}",
                        args.len()
                    ),));
                }
                let condition = match args[0] {
                    Value::Bool(flag) => flag,
                    _ => {
                        bail!(VmError::Runtime(
                            "assert condition must be a Bool".to_string(),
                        ))
                    }
                };
                let message = if args.len() == 2 {
                    match &args[1] {
                        Value::String(text) => text.clone(),
                        _ => {
                            bail!(VmError::Runtime(
                                "assert message must be a String".to_string(),
                            ))
                        }
                    }
                } else {
                    "assertion failed".to_string()
                };
                if !condition {
                    return Err(VmError::Runtime(message).into());
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::AssertEq => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "assert_eq expected 2 arguments but got {}",
                        args.len()
                    ),));
                }
                if args[0] != args[1] {
                    let left = args[0].to_string();
                    let right = args[1].to_string();
                    return Err(VmError::Runtime(format!(
                        "assert_eq failed: left {left} != right {right}"
                    ))
                    .into());
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::AssertNe => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "assert_ne expected 2 arguments but got {}",
                        args.len()
                    ),));
                }
                if args[0] == args[1] {
                    let value = args[0].to_string();
                    return Err(VmError::Runtime(format!(
                        "assert_ne failed: both sides evaluate to {value}"
                    ))
                    .into());
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::AssertFail => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "fail expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                let message = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "fail expects a String message".to_string(),
                        ))
                    }
                };
                return Err(VmError::Runtime(message).into());
            }
            StdFunctionKind::AssertSnapshot => {
                if !(2..=3).contains(&args.len()) {
                    bail!(VmError::Runtime(format!(
                        "assert_snapshot expected 2 or 3 arguments but got {}",
                        args.len()
                    )));
                }
                let name = self.expect_string(&args[0], "assert_snapshot name must be a String")?;
                let actual =
                    self.expect_string(&args[1], "assert_snapshot value must be a String")?;
                let label = if args.len() == 3 {
                    Some(self.expect_string(&args[2], "snapshot label must be a String")?)
                } else {
                    None
                };
                self.handle_snapshot_assertion(&name, &actual, label.as_deref())?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::AssertEmpty => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "assert_empty expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let value =
                    self.expect_string(&args[0], "assert_empty expects a String argument")?;
                if !value.is_empty() {
                    return Err(VmError::Runtime(format!(
                        "assert_empty failed: expected empty string but found '{value}'"
                    ))
                    .into());
                }
                self.stack.push(Value::Void);
            }

            // All other functions are handled by intrinsics
            _ => {
                bail!(VmError::Runtime(format!(
                    "Unknown builtin function kind: {:?}",
                    kind
                )))
            }
        }
        Ok(())
    }

    pub fn expect_string(&self, value: &Value, context: &str) -> Result<String> {
        match value {
            Value::String(text) => Ok(text.clone()),
            _ => bail!(VmError::Runtime(context.to_string())),
        }
    }

    fn expect_string_list(&self, value: &Value, context: &str) -> Result<Vec<String>> {
        match value {
            Value::List(items) => {
                let mut result = Vec::with_capacity(items.len());
                for item in items.iter() {
                    result.push(self.expect_string(item, context)?);
                }
                Ok(result)
            }
            _ => bail!(VmError::Runtime(context.to_string())),
        }
    }

    // Methods for removed functionality - kept for potential future use
    #[allow(dead_code)]
    fn expect_string_dict(&self, value: &Value, context: &str) -> Result<HashMap<String, String>> {
        match value {
            Value::Dict(entries) => {
                let mut map = HashMap::with_capacity(entries.len());
                for (key, entry_value) in entries.iter() {
                    let string_value = self.expect_string(entry_value, context)?;
                    map.insert(key.clone(), string_value);
                }
                Ok(map)
            }
            Value::Nil => Ok(HashMap::new()),
            _ => bail!(VmError::Runtime(context.to_string())),
        }
    }

    fn path_to_string(&self, path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    fn compute_dirname(&self, input: &str) -> String {
        let path = Path::new(input);
        match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => self.path_to_string(parent),
            Some(_) => {
                if path.has_root() {
                    self.path_to_string(path)
                } else {
                    ".".to_string()
                }
            }
            None => {
                if path.has_root() {
                    self.path_to_string(path)
                } else {
                    ".".to_string()
                }
            }
        }
    }

    fn compute_basename(&self, input: &str) -> String {
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

    fn compute_absolute(&self, target: &str, base: Option<String>) -> Result<String> {
        let target_path = PathBuf::from(target);
        if target_path.is_absolute() {
            let cleaned = target_path.clean();
            return Ok(self.path_to_string(cleaned.as_path()));
        }

        let mut base_path = if let Some(base) = base {
            PathBuf::from(base)
        } else {
            env::current_dir().map_err(|error| {
                VmError::Runtime(format!("failed to resolve current directory: {error}"))
            })?
        };

        if !base_path.is_absolute() {
            let cwd = env::current_dir().map_err(|error| {
                VmError::Runtime(format!("failed to resolve current directory: {error}"))
            })?;
            base_path = cwd.join(base_path);
        }

        let combined = base_path.join(target_path);
        let cleaned = combined.clean();
        Ok(self.path_to_string(cleaned.as_path()))
    }

    fn compute_relative(&self, target: &str, base: &str) -> Result<String> {
        let target_path = Path::new(target).clean();
        let base_path = Path::new(base).clean();
        match diff_paths(&target_path, &base_path) {
            Some(diff) => {
                if diff.as_os_str().is_empty() {
                    Ok(".".to_string())
                } else {
                    Ok(self.path_to_string(diff.as_path()))
                }
            }
            None => bail!(VmError::Runtime(format!(
                "unable to compute relative path from '{}' to '{}'",
                base, target
            ))),
        }
    }

    #[allow(dead_code)]
    fn make_process_result(
        &self,
        command: String,
        exit: i64,
        stdout: String,
        stderr: String,
    ) -> Value {
        Value::Struct(Rc::new(StructInstance {
            template: Rc::clone(&self.process_result_template),
            fields: vec![
                Value::Int(exit),
                Value::Bool(exit == 0),
                Value::String(stdout),
                Value::String(stderr),
                Value::String(command),
            ],
        }))
    }

    #[allow(dead_code)]
    fn read_from_pipe<R: Read>(
        reader: &mut Option<BufReader<R>>,
        size: Option<usize>,
    ) -> std::io::Result<String> {
        if let Some(ref mut pipe) = reader {
            let mut buffer = Vec::new();
            if let Some(limit) = size {
                let mut limited = pipe.take(limit as u64);
                limited.read_to_end(&mut buffer)?;
            } else {
                pipe.read_to_end(&mut buffer)?;
            }
            Ok(String::from_utf8_lossy(&buffer).to_string())
        } else {
            Ok(String::new())
        }
    }

    #[allow(dead_code)]
    fn make_cli_result(&self, exit: i64, stdout: String, stderr: String) -> Value {
        Value::Struct(Rc::new(StructInstance {
            template: Rc::clone(&self.cli_result_template),
            fields: vec![
                Value::Int(exit),
                Value::String(stdout),
                Value::String(stderr),
            ],
        }))
    }

    #[allow(dead_code)]
    fn make_cli_parse_result(&self, outcome: &CliParseOutcome) -> Value {
        let options = Self::dict_from_map(&outcome.options);
        let positionals = Self::dict_from_map(&outcome.positionals);
        let scopes = Self::scopes_to_value(&outcome.scopes);
        let path = Self::list_from_strings(&outcome.path);
        let rest = Self::list_from_strings(&outcome.rest);

        Value::Struct(Rc::new(StructInstance {
            template: Rc::clone(&self.cli_parse_result_template),
            fields: vec![
                Value::Bool(outcome.ok),
                Value::Int(outcome.exit),
                Value::String(outcome.command.clone()),
                path,
                options,
                positionals,
                scopes,
                rest,
                Value::String(outcome.message.clone()),
                Value::String(outcome.help.clone()),
            ],
        }))
    }

    #[allow(dead_code)]
    fn dict_from_map(map: &HashMap<String, Value>) -> Value {
        Value::Dict(Rc::new(
            map.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ))
    }

    #[allow(dead_code)]
    fn list_from_strings(items: &[String]) -> Value {
        Value::List(Rc::new(
            items.iter().cloned().map(Value::String).collect::<Vec<_>>(),
        ))
    }

    #[allow(dead_code)]
    fn scopes_to_value(scopes: &[CliScopeOutcome]) -> Value {
        let items: Vec<Value> = scopes
            .iter()
            .map(|scope| {
                let mut map = HashMap::new();
                map.insert("name".to_string(), Value::String(scope.name.clone()));
                map.insert("options".to_string(), Self::dict_from_map(&scope.options));
                map.insert(
                    "positionals".to_string(),
                    Self::dict_from_map(&scope.positionals),
                );
                Value::Dict(Rc::new(map))
            })
            .collect();
        Value::List(Rc::new(items))
    }

    pub fn handle_snapshot_assertion(
        &self,
        name: &str,
        actual: &str,
        label: Option<&str>,
    ) -> Result<()> {
        let settings = self.snapshot_settings.as_ref().ok_or_else(|| {
            VmError::Runtime(
                "assert_snapshot is only available when running via `tea test`".to_string(),
            )
        })?;
        let path = self.snapshot_file_path(name, label)?;

        if !path.exists() {
            if settings.update {
                fs::write(&path, actual).map_err(|error| {
                    VmError::Runtime(format!(
                        "failed to write snapshot '{}': {}",
                        path.display(),
                        error
                    ))
                })?;
                return Ok(());
            }
            return Err(VmError::Runtime(format!(
                "snapshot '{}' not found at {} (run `tea test --update-snapshots` to create it)",
                name,
                path.display()
            ))
            .into());
        }

        if settings.update {
            fs::write(&path, actual).map_err(|error| {
                VmError::Runtime(format!(
                    "failed to update snapshot '{}': {}",
                    path.display(),
                    error
                ))
            })?;
            return Ok(());
        }

        let expected = fs::read_to_string(&path).map_err(|error| {
            VmError::Runtime(format!(
                "failed to read snapshot '{}': {}",
                path.display(),
                error
            ))
        })?;

        if expected == actual {
            return Ok(());
        }

        let diff = Self::first_difference(&expected, actual);
        let mut message = format!("snapshot '{}' mismatch (file {})", name, path.display());
        if let Some((line, expected_line, actual_line)) = diff {
            message.push_str(&format!("\n  line {line}"));
            let expected_text = expected_line.unwrap_or_else(|| "<missing>".to_string());
            let actual_text = actual_line.unwrap_or_else(|| "<missing>".to_string());
            message.push_str(&format!("\n    expected: {}", expected_text));
            message.push_str(&format!("\n    actual: {}", actual_text));
        }
        message.push_str("\n(run `tea test --update-snapshots` to accept new output)");
        Err(VmError::Runtime(message).into())
    }

    fn snapshot_file_path(&self, name: &str, label: Option<&str>) -> Result<PathBuf> {
        let settings = self.snapshot_settings.as_ref().ok_or_else(|| {
            VmError::Runtime(
                "assert_snapshot is only available when running via `tea test`".to_string(),
            )
        })?;

        let mut dir = settings.snapshot_root.clone();
        if let Some(parent) = settings.relative_test_path.parent() {
            if !parent.as_os_str().is_empty() {
                dir.push(parent);
            }
        }
        if let Some(stem) = settings.relative_test_path.file_stem() {
            dir.push(stem);
        }

        fs::create_dir_all(&dir).map_err(|error| {
            VmError::Runtime(format!(
                "failed to create snapshot directory '{}': {}",
                dir.display(),
                error
            ))
        })?;

        let mut filename = Self::slugify(name);
        if let Some(label) = label {
            let label_slug = Self::slugify(label);
            if !label_slug.is_empty() {
                filename.push('_');
                filename.push_str(&label_slug);
            }
        }
        filename.push_str(".snap");

        Ok(dir.join(filename))
    }

    fn perform_numeric_binary<FI, FF>(&mut self, int_op: FI, float_op: FF) -> Result<()>
    where
        FI: FnOnce(i64, i64) -> Result<Value>,
        FF: FnOnce(f64, f64) -> Result<Value>,
    {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = match (left, right) {
            (Value::Int(a), Value::Int(b)) => int_op(a, b)?,
            (Value::Float(a), Value::Float(b)) => float_op(a, b)?,
            (Value::Int(a), Value::Float(b)) => float_op(a as f64, b)?,
            (Value::Float(a), Value::Int(b)) => float_op(a, b as f64)?,
            _ => {
                return Err(VmError::Runtime(
                    "binary operation expects numeric operands".to_string(),
                )
                .into());
            }
        };
        self.stack.push(result);
        Ok(())
    }

    fn value_matches_type(&self, value: &Value, type_check: &TypeCheck) -> bool {
        match type_check {
            TypeCheck::Bool => matches!(value, Value::Bool(_)),
            TypeCheck::Int => matches!(value, Value::Int(_)),
            TypeCheck::Float => matches!(value, Value::Float(_)),
            TypeCheck::String => matches!(value, Value::String(_)),
            TypeCheck::Nil => matches!(value, Value::Nil),
            TypeCheck::Struct(name) => match value {
                Value::Struct(instance) => instance.template.name == *name,
                _ => false,
            },
            TypeCheck::Enum(name) => match value {
                Value::EnumVariant(variant) => variant.enum_name == *name,
                _ => false,
            },
            TypeCheck::Error {
                error_name,
                variant_name,
            } => match value {
                Value::Error(error) => {
                    if &error.error_name != error_name {
                        false
                    } else {
                        match variant_name {
                            Some(name) => &error.variant_name == name,
                            None => true,
                        }
                    }
                }
                _ => false,
            },
            TypeCheck::Optional(inner) => {
                matches!(value, Value::Nil) || self.value_matches_type(value, inner)
            }
            TypeCheck::Union(members) => members
                .iter()
                .any(|member| self.value_matches_type(value, member)),
        }
    }

    fn slugify(input: &str) -> String {
        let mut result = String::new();
        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                result.push(ch.to_ascii_lowercase());
            } else if !result.ends_with('_') {
                result.push('_');
            }
        }
        let trimmed = result.trim_matches('_').to_string();
        if trimmed.is_empty() {
            "snapshot".to_string()
        } else {
            trimmed
        }
    }

    fn first_difference(
        expected: &str,
        actual: &str,
    ) -> Option<(usize, Option<String>, Option<String>)> {
        let mut line = 1usize;
        let mut exp_iter = expected.lines();
        let mut act_iter = actual.lines();

        loop {
            match (exp_iter.next(), act_iter.next()) {
                (Some(exp), Some(act)) => {
                    if exp != act {
                        return Some((line, Some(exp.to_string()), Some(act.to_string())));
                    }
                }
                (Some(exp), None) => return Some((line, Some(exp.to_string()), None)),
                (None, Some(act)) => return Some((line, None, Some(act.to_string()))),
                (None, None) => break,
            }
            line += 1;
        }

        if expected.ends_with('\n') != actual.ends_with('\n') {
            let expected_line = if expected.ends_with('\n') {
                Some("<newline>".to_string())
            } else {
                None
            };
            let actual_line = if actual.ends_with('\n') {
                Some("<newline>".to_string())
            } else {
                None
            };
            return Some((line, expected_line, actual_line));
        }

        None
    }

    fn perform_numeric_comparison<FI, FF>(&mut self, int_op: FI, float_op: FF) -> Result<()>
    where
        FI: FnOnce(i64, i64) -> bool,
        FF: FnOnce(f64, f64) -> bool,
    {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = match (left, right) {
            (Value::Int(a), Value::Int(b)) => int_op(a, b),
            (Value::Float(a), Value::Float(b)) => float_op(a, b),
            (Value::Int(a), Value::Float(b)) => float_op(a as f64, b),
            (Value::Float(a), Value::Int(b)) => float_op(a, b as f64),
            _ => {
                return Err(VmError::Runtime(
                    "comparison operation expects numeric operands".to_string(),
                )
                .into());
            }
        };
        self.stack.push(Value::Bool(result));
        Ok(())
    }

    fn call_function(&mut self, arg_count: usize) -> Result<()> {
        let callee_index = self
            .stack
            .len()
            .checked_sub(arg_count + 1)
            .ok_or_else(|| VmError::Runtime("call stack underflow".to_string()))?;

        let function_value = self
            .stack
            .get(callee_index)
            .cloned()
            .ok_or_else(|| VmError::Runtime("missing function value".to_string()))?;

        let (function_index, captures) = match function_value {
            Value::Function(index) => (index, None),
            Value::Closure(closure) => (closure.function_index, Some(closure.captures.clone())),
            _ => {
                bail!(VmError::Runtime(
                    "attempted to call a non-function".to_string()
                ))
            }
        };

        let function = self
            .program
            .functions
            .get(function_index)
            .ok_or_else(|| VmError::Runtime("function index out of bounds".to_string()))?;

        if function.arity != arg_count {
            bail!(VmError::Runtime(format!(
                "expected {} arguments but got {}",
                function.arity, arg_count
            )));
        }

        if let Some(captures) = captures {
            let capture_values = captures.as_ref();
            for (offset, value) in capture_values.iter().enumerate() {
                self.stack.insert(callee_index + 1 + offset, value.clone());
            }
        }

        self.frames.push(Frame {
            chunk: ChunkRef::Function(function_index),
            ip: 0,
            stack_start: callee_index,
        });

        Ok(())
    }

    fn resolve_chunk(&self, chunk: &ChunkRef) -> &super::bytecode::Chunk {
        match chunk {
            ChunkRef::Main => &self.program.chunk,
            ChunkRef::Function(index) => &self.program.functions[*index].chunk,
        }
    }

    fn local_index(&self, frame_index: usize, index: usize) -> Result<usize> {
        let frame = self
            .frames
            .get(frame_index)
            .ok_or_else(|| VmError::Runtime("no active frame".to_string()))?;
        Ok(frame.stack_start + 1 + index)
    }
}

// JSON helper functions - kept for potential future use
#[allow(dead_code)]
fn value_to_json(value: &Value) -> Result<JsonValue, VmError> {
    Ok(match value {
        Value::Nil => JsonValue::Null,
        Value::Void => JsonValue::Null,
        Value::Int(v) => (*v).into(),
        Value::Float(v) => serde_json::Number::from_f64(*v)
            .map(JsonValue::Number)
            .ok_or_else(|| {
                VmError::Runtime("cannot encode NaN or infinite floats to JSON".to_string())
            })?,
        Value::Bool(v) => JsonValue::Bool(*v),
        Value::String(text) => JsonValue::String(text.clone()),
        Value::List(values) => {
            let mut items = Vec::with_capacity(values.len());
            for item in values.iter() {
                items.push(value_to_json(item)?);
            }
            JsonValue::Array(items)
        }
        Value::Dict(entries) => {
            let mut object = serde_json::Map::with_capacity(entries.len());
            for (key, entry_value) in entries.iter() {
                object.insert(key.clone(), value_to_json(entry_value)?);
            }
            JsonValue::Object(object)
        }
        Value::Struct(instance) => {
            let mut object = serde_json::Map::with_capacity(instance.template.field_names.len());
            for (field_name, field_value) in instance
                .template
                .field_names
                .iter()
                .zip(instance.fields.iter())
            {
                object.insert(field_name.clone(), value_to_json(field_value)?);
            }
            JsonValue::Object(object)
        }
        Value::EnumVariant(variant) => {
            JsonValue::String(format!("{}.{}", variant.enum_name, variant.variant_name))
        }
        Value::Error(error) => JsonValue::String(Value::Error(error.clone()).to_string()),
        Value::Function(_) | Value::Closure(_) => {
            return Err(VmError::Runtime(
                "json encode does not support functions or closures".to_string(),
            ));
        }
    })
}

#[allow(dead_code)]
fn json_to_value(json: &JsonValue) -> Result<Value, VmError> {
    Ok(match json {
        JsonValue::Null => Value::Nil,
        JsonValue::Bool(v) => Value::Bool(*v),
        JsonValue::Number(number) => {
            if let Some(int) = number.as_i64() {
                Value::Int(int)
            } else if let Some(uint) = number.as_u64() {
                if uint <= i64::MAX as u64 {
                    Value::Int(uint as i64)
                } else if let Some(float) = number.as_f64() {
                    Value::Float(float)
                } else {
                    return Err(VmError::Runtime(
                        "JSON number is too large to fit in Tea numeric types".to_string(),
                    ));
                }
            } else if let Some(float) = number.as_f64() {
                Value::Float(float)
            } else {
                return Err(VmError::Runtime(
                    "Unsupported JSON number representation".to_string(),
                ));
            }
        }
        JsonValue::String(text) => Value::String(text.clone()),
        JsonValue::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(json_to_value(item)?);
            }
            Value::List(Rc::new(values))
        }
        JsonValue::Object(map) => {
            let mut entries = HashMap::with_capacity(map.len());
            for (key, value) in map.iter() {
                entries.insert(key.clone(), json_to_value(value)?);
            }
            Value::Dict(Rc::new(entries))
        }
    })
}
