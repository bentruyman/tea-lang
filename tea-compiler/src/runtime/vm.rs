use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::rc::Rc;
use std::time::UNIX_EPOCH;

use dirs_next::{config_dir, home_dir};
use glob::glob;
use path_clean::PathClean;
use pathdiff::diff_paths;
use tea_support::{cli_error, cli_target_error, env_error, fs_error, io_error, process_error};
use tempfile::NamedTempFile;
use walkdir::WalkDir;

use super::bytecode::{Instruction, Program, TypeCheck};
use super::cli::{parse_cli, CliParseOutcome, CliScopeOutcome};
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
    next_fs_handle: i64,
    fs_handles: HashMap<i64, FsReadHandle>,
    next_process_handle: i64,
    process_handles: HashMap<i64, ProcessEntry>,
    snapshot_settings: Option<SnapshotSettings>,
    cli_result_template: Rc<StructTemplate>,
    cli_parse_result_template: Rc<StructTemplate>,
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

struct FsReadHandle {
    reader: BufReader<File>,
}

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

fn vm_collect_modified(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
}

#[cfg(unix)]
fn vm_metadata_mode(metadata: &fs::Metadata) -> i64 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() as i64
}

#[cfg(windows)]
fn vm_metadata_mode(metadata: &fs::Metadata) -> i64 {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() as i64
}

#[cfg(not(any(unix, windows)))]
fn vm_metadata_mode(metadata: &fs::Metadata) -> i64 {
    let _ = metadata;
    0
}

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
                    } else {
                        // Handle numeric addition
                        let result = match (left, right) {
                            (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                            (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                            (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 + b),
                            (Value::Float(a), Value::Int(b)) => Value::Float(a + b as f64),
                            _ => {
                                return Err(VmError::Runtime(
                                    "addition requires numeric or string operands".to_string(),
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
            StdFunctionKind::UtilLen => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "len expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                let length = match &args[0] {
                    Value::String(text) => text.chars().count() as i64,
                    Value::List(items) => items.len() as i64,
                    _ => {
                        bail!(VmError::Runtime("len expects a String or List".to_string(),))
                    }
                };
                self.stack.push(Value::Int(length));
            }
            StdFunctionKind::UtilToString => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "to_string expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack.push(Value::String(args[0].to_string()));
            }
            StdFunctionKind::UtilClampInt => {
                if args.len() != 3 {
                    bail!(VmError::Runtime(format!(
                        "clamp_int expected 3 arguments but got {}",
                        args.len()
                    ),));
                }
                let value = match args[0] {
                    Value::Int(v) => v,
                    _ => {
                        bail!(VmError::Runtime(
                            "clamp_int value must be an Int".to_string(),
                        ))
                    }
                };
                let min = match args[1] {
                    Value::Int(v) => v,
                    _ => {
                        bail!(VmError::Runtime(
                            "clamp_int minimum must be an Int".to_string(),
                        ))
                    }
                };
                let max = match args[2] {
                    Value::Int(v) => v,
                    _ => {
                        bail!(VmError::Runtime(
                            "clamp_int maximum must be an Int".to_string(),
                        ))
                    }
                };
                let clamped = if min > max {
                    bail!(VmError::Runtime(
                        "clamp_int expects minimum to be <= maximum".to_string(),
                    ));
                } else {
                    value.clamp(min, max)
                };
                self.stack.push(Value::Int(clamped));
            }
            StdFunctionKind::UtilIsNil => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_nil expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack.push(Value::Bool(matches!(args[0], Value::Nil)));
            }
            StdFunctionKind::UtilIsBool => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_bool expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::Bool(_))));
            }
            StdFunctionKind::UtilIsInt => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_int expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::Int(_))));
            }
            StdFunctionKind::UtilIsFloat => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_float expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::Float(_))));
            }
            StdFunctionKind::UtilIsString => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_string expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::String(_))));
            }
            StdFunctionKind::UtilIsList => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_list expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::List(_))));
            }
            StdFunctionKind::UtilIsStruct => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_struct expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::Struct(_))));
            }
            StdFunctionKind::UtilIsError => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_error expected 1 argument but got {}",
                        args.len()
                    ),));
                }
                self.stack
                    .push(Value::Bool(matches!(args[0], Value::Error(_))));
            }
            StdFunctionKind::EnvGet => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "env.get expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let key =
                    self.expect_string(&args[0], "env.get expects the name to be a String")?;
                let value = env::var(&key).unwrap_or_default();
                self.stack.push(Value::String(value));
            }
            StdFunctionKind::EnvGetOr => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "env.get_or expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let key =
                    self.expect_string(&args[0], "env.get_or expects the name to be a String")?;
                let fallback =
                    self.expect_string(&args[1], "env.get_or expects the fallback to be a String")?;
                let value = env::var(&key).unwrap_or(fallback);
                self.stack.push(Value::String(value));
            }
            StdFunctionKind::EnvHas => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "env.has expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let key =
                    self.expect_string(&args[0], "env.has expects the name to be a String")?;
                self.stack.push(Value::Bool(env::var_os(&key).is_some()));
            }
            StdFunctionKind::EnvRequire => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "env.require expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let key =
                    self.expect_string(&args[0], "env.require expects the name to be a String")?;
                match env::var(&key) {
                    Ok(value) => self.stack.push(Value::String(value)),
                    Err(_) => {
                        return Err(VmError::Runtime(env_error(
                            "require",
                            Some(&key),
                            "variable not set",
                        ))
                        .into());
                    }
                }
            }
            StdFunctionKind::EnvSet => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "env.set expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let key =
                    self.expect_string(&args[0], "env.set expects the name to be a String")?;
                let value =
                    self.expect_string(&args[1], "env.set expects the value to be a String")?;
                env::set_var(&key, &value);
                self.stack.push(Value::Void);
            }
            StdFunctionKind::EnvUnset => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "env.unset expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let key =
                    self.expect_string(&args[0], "env.unset expects the name to be a String")?;
                env::remove_var(&key);
                self.stack.push(Value::Void);
            }
            StdFunctionKind::EnvVars => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "env.vars expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let mut map = HashMap::new();
                for (key, value) in env::vars() {
                    map.insert(key, Value::String(value));
                }
                self.stack.push(Value::Dict(Rc::new(map)));
            }
            StdFunctionKind::EnvCwd => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "env.cwd expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                match env::current_dir() {
                    Ok(path) => self
                        .stack
                        .push(Value::String(path.to_string_lossy().into_owned())),
                    Err(error) => {
                        return Err(VmError::Runtime(env_error("cwd", None, error)).into());
                    }
                }
            }
            StdFunctionKind::EnvSetCwd => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "env.set_cwd expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path =
                    self.expect_string(&args[0], "env.set_cwd expects the path to be a String")?;
                if let Err(error) = env::set_current_dir(&path) {
                    return Err(VmError::Runtime(env_error("set_cwd", Some(&path), error)).into());
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::EnvTempDir => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "env.temp_dir expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let tmp = env::temp_dir();
                self.stack
                    .push(Value::String(tmp.to_string_lossy().into_owned()));
            }
            StdFunctionKind::EnvHomeDir => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "env.home_dir expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let value = home_dir()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.stack.push(Value::String(value));
            }
            StdFunctionKind::EnvConfigDir => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "env.config_dir expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let value = config_dir()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.stack.push(Value::String(value));
            }
            StdFunctionKind::PathSeparator => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "separator expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                self.stack
                    .push(Value::String(std::path::MAIN_SEPARATOR.to_string()));
            }
            StdFunctionKind::PathIsAbsolute => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_absolute expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self.expect_string(&args[0], "is_absolute expects a String argument")?;
                self.stack.push(Value::Bool(Path::new(&path).is_absolute()));
            }
            StdFunctionKind::PathJoin => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "join expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let parts =
                    self.expect_string_list(&args[0], "join expects a List[String] argument")?;
                let mut buffer = PathBuf::new();
                for part in parts {
                    buffer.push(part);
                }
                self.stack
                    .push(Value::String(self.path_to_string(buffer.as_path())));
            }
            StdFunctionKind::PathComponents => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "components expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self.expect_string(&args[0], "components expects a String argument")?;
                let mut values = Vec::new();
                for component in Path::new(&path).components() {
                    values.push(Value::String(
                        component.as_os_str().to_string_lossy().into_owned(),
                    ));
                }
                self.stack.push(Value::List(Rc::new(values)));
            }
            StdFunctionKind::PathDirname => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "dirname expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self.expect_string(&args[0], "dirname expects a String argument")?;
                let dirname = self.compute_dirname(&path);
                self.stack.push(Value::String(dirname));
            }
            StdFunctionKind::PathBasename => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "basename expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self.expect_string(&args[0], "basename expects a String argument")?;
                let basename = self.compute_basename(&path);
                self.stack.push(Value::String(basename));
            }
            StdFunctionKind::PathExtension => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "extension expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self.expect_string(&args[0], "extension expects a String argument")?;
                let ext = Path::new(&path)
                    .extension()
                    .map(|ext| ext.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.stack.push(Value::String(ext));
            }
            StdFunctionKind::PathSetExtension => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "set_extension expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let path =
                    self.expect_string(&args[0], "set_extension expects the path to be a String")?;
                let extension = self.expect_string(
                    &args[1],
                    "set_extension expects the extension to be a String",
                )?;
                let mut buf = PathBuf::from(path);
                buf.set_extension(extension);
                self.stack
                    .push(Value::String(self.path_to_string(buf.as_path())));
            }
            StdFunctionKind::PathStripExtension => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "strip_extension expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self
                    .expect_string(&args[0], "strip_extension expects the path to be a String")?;
                let mut buf = PathBuf::from(path);
                buf.set_extension("");
                self.stack
                    .push(Value::String(self.path_to_string(buf.as_path())));
            }
            StdFunctionKind::PathNormalize => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "normalize expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = self.expect_string(&args[0], "normalize expects a String argument")?;
                let normalized = Path::new(&path).clean();
                self.stack
                    .push(Value::String(self.path_to_string(normalized.as_path())));
            }
            StdFunctionKind::PathAbsolute => {
                if !(1..=2).contains(&args.len()) {
                    bail!(VmError::Runtime(format!(
                        "absolute expected 1 or 2 arguments but got {}",
                        args.len()
                    )));
                }
                let target =
                    self.expect_string(&args[0], "absolute expects the path to be a String")?;
                let base = if args.len() == 2 {
                    Some(self.expect_string(&args[1], "absolute expects the base to be a String")?)
                } else {
                    None
                };
                let resolved = self.compute_absolute(&target, base)?;
                self.stack.push(Value::String(resolved));
            }
            StdFunctionKind::PathRelative => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "relative expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let target =
                    self.expect_string(&args[0], "relative expects the target to be a String")?;
                let base =
                    self.expect_string(&args[1], "relative expects the base to be a String")?;
                let relative = self.compute_relative(&target, &base)?;
                self.stack.push(Value::String(relative));
            }
            StdFunctionKind::IoReadLine => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "read_line expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let mut buffer = String::new();
                let bytes_read = std::io::stdin()
                    .read_line(&mut buffer)
                    .map_err(|error| VmError::Runtime(io_error("read_line", &error)))?;
                if bytes_read == 0 {
                    self.stack.push(Value::Nil);
                } else {
                    if buffer.ends_with('\n') {
                        buffer.pop();
                        if buffer.ends_with('\r') {
                            buffer.pop();
                        }
                    }
                    self.stack.push(Value::String(buffer));
                }
            }
            StdFunctionKind::IoReadAll => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "read_all expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .map_err(|error| VmError::Runtime(io_error("read_all", &error)))?;
                self.stack.push(Value::String(buffer));
            }
            StdFunctionKind::IoReadBytes => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "read_bytes expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let mut buffer = Vec::new();
                std::io::stdin()
                    .read_to_end(&mut buffer)
                    .map_err(|error| VmError::Runtime(io_error("read_bytes", &error)))?;
                let values: Vec<Value> = buffer
                    .into_iter()
                    .map(|byte| Value::Int(byte as i64))
                    .collect();
                self.stack.push(Value::List(Rc::new(values)));
            }
            StdFunctionKind::IoWrite => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "write expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let text = match &args[0] {
                    Value::String(text) => text.as_bytes(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write expects a String argument".to_string()
                        ))
                    }
                };
                std::io::stdout()
                    .write_all(text)
                    .map_err(|error| VmError::Runtime(io_error("write", &error)))?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::IoWriteErr => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "write_err expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let text = match &args[0] {
                    Value::String(text) => text.as_bytes(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_err expects a String argument".to_string()
                        ))
                    }
                };
                std::io::stderr()
                    .write_all(text)
                    .map_err(|error| VmError::Runtime(io_error("write_err", &error)))?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::IoFlush => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "flush expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                std::io::stdout()
                    .flush()
                    .map_err(|error| VmError::Runtime(io_error("flush", &error)))?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::JsonEncode => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "json.encode expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let json_value = value_to_json(&args[0])?;
                let encoded = serde_json::to_string(&json_value)
                    .map_err(|error| VmError::Runtime(format!("failed to encode JSON: {error}")))?;
                self.stack.push(Value::String(encoded));
            }
            StdFunctionKind::JsonDecode => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "json.decode expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let text = match &args[0] {
                    Value::String(text) => text,
                    _ => {
                        bail!(VmError::Runtime(
                            "json.decode expects the input to be a String".to_string()
                        ))
                    }
                };
                let parsed: JsonValue = serde_json::from_str(text)
                    .map_err(|error| VmError::Runtime(format!("failed to decode JSON: {error}")))?;
                let value = json_to_value(&parsed)?;
                self.stack.push(value);
            }
            StdFunctionKind::YamlEncode => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "yaml.encode expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let json_value = value_to_json(&args[0])?;
                let encoded = serde_yaml::to_string(&json_value)
                    .map_err(|error| VmError::Runtime(format!("failed to encode YAML: {error}")))?;
                self.stack.push(Value::String(encoded));
            }
            StdFunctionKind::YamlDecode => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "yaml.decode expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let text = match &args[0] {
                    Value::String(text) => text,
                    _ => {
                        bail!(VmError::Runtime(
                            "yaml.decode expects the input to be a String".to_string()
                        ))
                    }
                };
                let parsed: serde_yaml::Value = serde_yaml::from_str(text)
                    .map_err(|error| VmError::Runtime(format!("failed to decode YAML: {error}")))?;
                let json_value = serde_json::to_value(parsed).map_err(|error| {
                    VmError::Runtime(format!(
                        "failed to normalise YAML to JSON representation: {error}"
                    ))
                })?;
                let value = json_to_value(&json_value)?;
                self.stack.push(value);
            }
            StdFunctionKind::FsReadText => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "read_text expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "read_text expects a String path".to_string()
                        ))
                    }
                };
                let contents = std::fs::read_to_string(&path)
                    .map_err(|error| VmError::Runtime(fs_error("read_text", &path, &error)))?;
                self.stack.push(Value::String(contents));
            }
            StdFunctionKind::FsWriteText => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "write_text expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_text expects the path to be a String".to_string(),
                        ))
                    }
                };
                let contents = match &args[1] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_text expects the contents to be a String".to_string(),
                        ))
                    }
                };
                std::fs::write(&path, contents.as_bytes())
                    .map_err(|error| VmError::Runtime(fs_error("write_text", &path, &error)))?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsWriteTextAtomic => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "write_text_atomic expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_text_atomic expects the path to be a String".to_string(),
                        ))
                    }
                };
                let contents = match &args[1] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_text_atomic expects the contents to be a String".to_string(),
                        ))
                    }
                };
                vm_write_atomic(Path::new(&path), contents.as_bytes())?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsReadBytes => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "read_bytes expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "read_bytes expects a String path".to_string()
                        ))
                    }
                };
                let bytes = std::fs::read(&path)
                    .map_err(|error| VmError::Runtime(fs_error("read_bytes", &path, &error)))?;
                let values: Vec<Value> = bytes
                    .into_iter()
                    .map(|byte| Value::Int(byte as i64))
                    .collect();
                self.stack.push(Value::List(Rc::new(values)));
            }
            StdFunctionKind::FsWriteBytes => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "write_bytes expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_bytes expects the path to be a String".to_string(),
                        ))
                    }
                };
                let list = match &args[1] {
                    Value::List(items) => items.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_bytes expects a List of Int values".to_string(),
                        ))
                    }
                };
                let mut buffer = Vec::with_capacity(list.len());
                for (index, value) in list.iter().enumerate() {
                    match value {
                        Value::Int(byte) if (0..=255).contains(byte) => {
                            buffer.push(*byte as u8);
                        }
                        Value::Int(_) => {
                            bail!(VmError::Runtime(format!(
                                "write_bytes expects byte values between 0 and 255 (argument index {index})"
                            )))
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "write_bytes expects a List[Int]".to_string(),
                            ))
                        }
                    }
                }
                std::fs::write(&path, buffer)
                    .map_err(|error| VmError::Runtime(fs_error("write_bytes", &path, &error)))?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsWriteBytesAtomic => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "write_bytes_atomic expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_bytes_atomic expects the path to be a String".to_string(),
                        ))
                    }
                };
                let list = match &args[1] {
                    Value::List(items) => items.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "write_bytes_atomic expects a List of Int values".to_string(),
                        ))
                    }
                };
                let mut buffer = Vec::with_capacity(list.len());
                for (index, value) in list.iter().enumerate() {
                    match value {
                        Value::Int(byte) if (0..=255).contains(byte) => {
                            buffer.push(*byte as u8);
                        }
                        Value::Int(_) => {
                            bail!(VmError::Runtime(format!(
                                "write_bytes_atomic expects byte values between 0 and 255 (argument index {index})"
                            )))
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "write_bytes_atomic expects a List[Int]".to_string(),
                            ))
                        }
                    }
                }
                vm_write_atomic(Path::new(&path), &buffer)?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsCreateDir => {
                if !(1..=2).contains(&args.len()) {
                    bail!(VmError::Runtime(format!(
                        "create_dir expected 1 or 2 arguments but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "create_dir expects the path to be a String".to_string(),
                        ))
                    }
                };
                let recursive = if args.len() == 2 {
                    match args[1] {
                        Value::Bool(flag) => flag,
                        _ => {
                            bail!(VmError::Runtime(
                                "create_dir optional second argument must be a Bool".to_string(),
                            ))
                        }
                    }
                } else {
                    false
                };

                if recursive {
                    std::fs::create_dir_all(&path)
                        .map_err(|error| VmError::Runtime(fs_error("create_dir", &path, &error)))?;
                } else {
                    std::fs::create_dir(&path)
                        .map_err(|error| VmError::Runtime(fs_error("create_dir", &path, &error)))?;
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsEnsureDir => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "ensure_dir expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "ensure_dir expects the path to be a String".to_string(),
                        ))
                    }
                };
                fs::create_dir_all(&path)
                    .map_err(|error| VmError::Runtime(fs_error("ensure_dir", &path, &error)))?;
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsEnsureParent => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "ensure_parent expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "ensure_parent expects the path to be a String".to_string(),
                        ))
                    }
                };
                let fs_path = Path::new(&path);
                if let Some(parent) = fs_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent).map_err(|error| {
                            VmError::Runtime(fs_error("ensure_parent", &path, &error))
                        })?;
                    }
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsRemove => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "remove expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path_text = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "remove expects the path to be a String".to_string(),
                        ))
                    }
                };
                let path = Path::new(&path_text);
                if path.is_dir() {
                    std::fs::remove_dir_all(path).map_err(|error| {
                        VmError::Runtime(fs_error("remove", &path_text, &error))
                    })?;
                } else {
                    std::fs::remove_file(path).map_err(|error| {
                        VmError::Runtime(fs_error("remove", &path_text, &error))
                    })?;
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::FsExists => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "exists expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "exists expects the path to be a String".to_string(),
                        ))
                    }
                };
                self.stack.push(Value::Bool(Path::new(&path).exists()));
            }
            StdFunctionKind::FsIsDir => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_dir expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path_text = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "is_dir expects the path to be a String".to_string(),
                        ))
                    }
                };
                let is_dir = Path::new(&path_text).is_dir();
                self.stack.push(Value::Bool(is_dir));
            }
            StdFunctionKind::FsIsSymlink => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_symlink expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "is_symlink expects the path to be a String".to_string(),
                        ))
                    }
                };
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|error| VmError::Runtime(fs_error("is_symlink", &path, &error)))?;
                self.stack
                    .push(Value::Bool(metadata.file_type().is_symlink()));
            }
            StdFunctionKind::FsSize => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "size expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "size expects the path to be a String".to_string(),
                        ))
                    }
                };
                let metadata = std::fs::metadata(&path)
                    .map_err(|error| VmError::Runtime(fs_error("size", &path, &error)))?;
                self.stack.push(Value::Int(metadata.len() as i64));
            }
            StdFunctionKind::FsModified => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "modified expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "modified expects the path to be a String".to_string(),
                        ))
                    }
                };
                let metadata = std::fs::metadata(&path)
                    .map_err(|error| VmError::Runtime(fs_error("modified", &path, &error)))?;
                let modified_time = metadata
                    .modified()
                    .map_err(|error| VmError::Runtime(fs_error("modified", &path, &error)))?;
                let seconds = match modified_time.duration_since(UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(error) => {
                        bail!(VmError::Runtime(format!(
                            "modified time for '{path}' precedes Unix epoch: {error}"
                        )))
                    }
                };
                self.stack.push(Value::Int(seconds));
            }
            StdFunctionKind::FsPermissions => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "permissions expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "permissions expects the path to be a String".to_string(),
                        ))
                    }
                };
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|error| VmError::Runtime(fs_error("permissions", &path, &error)))?;
                self.stack.push(Value::Int(vm_metadata_mode(&metadata)));
            }
            StdFunctionKind::FsIsReadonly => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "is_readonly expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "is_readonly expects the path to be a String".to_string(),
                        ))
                    }
                };
                let metadata = std::fs::metadata(&path)
                    .map_err(|error| VmError::Runtime(fs_error("is_readonly", &path, &error)))?;
                self.stack
                    .push(Value::Bool(metadata.permissions().readonly()));
            }
            StdFunctionKind::FsListDir => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "list_dir expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "list_dir expects the path to be a String".to_string(),
                        ))
                    }
                };
                let mut entries = Vec::new();
                let dir = fs::read_dir(&path)
                    .map_err(|error| VmError::Runtime(fs_error("list_dir", &path, &error)))?;
                for entry in dir {
                    match entry {
                        Ok(dir_entry) => {
                            entries.push(dir_entry.path().to_string_lossy().into_owned());
                        }
                        Err(error) => {
                            bail!(VmError::Runtime(fs_error("list_dir", &path, &error)))
                        }
                    }
                }
                entries.sort();
                self.stack.push(vm_strings_to_list(entries));
            }
            StdFunctionKind::FsWalk => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "walk expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "walk expects the path to be a String".to_string(),
                        ))
                    }
                };
                let mut entries = Vec::new();
                for entry in WalkDir::new(&path) {
                    match entry {
                        Ok(dir_entry) => {
                            if dir_entry.depth() == 0 {
                                continue;
                            }
                            entries.push(dir_entry.path().to_string_lossy().into_owned());
                        }
                        Err(error) => {
                            bail!(VmError::Runtime(fs_error("walk", &path, &error)))
                        }
                    }
                }
                entries.sort();
                self.stack.push(vm_strings_to_list(entries));
            }
            StdFunctionKind::FsGlob => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "glob expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let pattern = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "glob expects the pattern to be a String".to_string(),
                        ))
                    }
                };
                let mut matches = Vec::new();
                let paths = glob(&pattern)
                    .map_err(|error| VmError::Runtime(fs_error("glob", &pattern, &error)))?;
                for entry in paths {
                    match entry {
                        Ok(path_buf) => matches.push(path_buf.to_string_lossy().into_owned()),
                        Err(error) => {
                            bail!(VmError::Runtime(fs_error("glob", &pattern, &error)))
                        }
                    }
                }
                matches.sort();
                self.stack.push(vm_strings_to_list(matches));
            }
            StdFunctionKind::FsMetadata => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "metadata expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "metadata expects the path to be a String".to_string(),
                        ))
                    }
                };
                let fs_path = PathBuf::from(&path);
                let metadata = fs::symlink_metadata(&fs_path)
                    .map_err(|error| VmError::Runtime(fs_error("metadata", &path, &error)))?;
                self.stack
                    .push(vm_metadata_value(fs_path.as_path(), metadata));
            }
            StdFunctionKind::FsOpenRead => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "open_read expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let path = match &args[0] {
                    Value::String(text) => text.clone(),
                    _ => {
                        bail!(VmError::Runtime(
                            "open_read expects the path to be a String".to_string(),
                        ))
                    }
                };
                let file = File::open(&path)
                    .map_err(|error| VmError::Runtime(fs_error("open_read", &path, &error)))?;
                let handle_id = self.next_fs_handle;
                self.next_fs_handle += 1;
                self.fs_handles.insert(
                    handle_id,
                    FsReadHandle {
                        reader: BufReader::new(file),
                    },
                );
                self.stack.push(Value::Int(handle_id));
            }
            StdFunctionKind::FsReadChunk => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "read_chunk expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "read_chunk expects the first argument to be a handle Int".to_string(),
                        ))
                    }
                };
                let size = match args[1] {
                    Value::Int(length) if length > 0 => length as usize,
                    Value::Int(_) => {
                        bail!(VmError::Runtime(
                            "read_chunk expects a positive chunk size".to_string(),
                        ))
                    }
                    _ => {
                        bail!(VmError::Runtime(
                            "read_chunk expects the chunk size to be an Int".to_string(),
                        ))
                    }
                };
                let handle = self
                    .fs_handles
                    .get_mut(&handle_id)
                    .ok_or_else(|| VmError::Runtime(format!("invalid file handle {handle_id}")))?;
                let mut buffer = vec![0u8; size];
                let bytes_read = handle.reader.read(&mut buffer).map_err(|error| {
                    let target = format!("handle {handle_id}");
                    VmError::Runtime(fs_error("read_chunk", &target, &error))
                })?;
                buffer.truncate(bytes_read);
                let values: Vec<Value> = buffer
                    .into_iter()
                    .map(|byte| Value::Int(byte as i64))
                    .collect();
                self.stack.push(Value::List(Rc::new(values)));
            }
            StdFunctionKind::FsClose => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "close expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "close expects the handle to be an Int".to_string(),
                        ))
                    }
                };
                if self.fs_handles.remove(&handle_id).is_none() {
                    bail!(VmError::Runtime(format!("invalid file handle {handle_id}")));
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::ProcessRun => {
                if args.is_empty() || args.len() > 5 {
                    bail!(VmError::Runtime(format!(
                        "process.run expected between 1 and 5 arguments but got {}",
                        args.len()
                    )));
                }
                let command =
                    self.expect_string(&args[0], "process.run expects the command to be a String")?;
                let arg_list = if args.len() >= 2 {
                    match &args[1] {
                        Value::Nil => Vec::new(),
                        value => self.expect_string_list(
                            value,
                            "process.run expects args to be a List[String]",
                        )?,
                    }
                } else {
                    Vec::new()
                };
                let env_map = if args.len() >= 3 {
                    match &args[2] {
                        Value::Nil => HashMap::new(),
                        value => self.expect_string_dict(
                            value,
                            "process.run expects env to be a Dict[String, String]",
                        )?,
                    }
                } else {
                    HashMap::new()
                };
                let cwd = if args.len() >= 4 {
                    match &args[3] {
                        Value::Nil => None,
                        value => Some(
                            self.expect_string(value, "process.run expects cwd to be a String")?,
                        ),
                    }
                } else {
                    None
                };
                let stdin_data = if args.len() >= 5 {
                    match &args[4] {
                        Value::Nil => None,
                        Value::String(text) => Some(text.clone()),
                        _ => {
                            bail!(VmError::Runtime(
                                "process.run expects stdin to be a String when provided"
                                    .to_string()
                            ))
                        }
                    }
                } else {
                    None
                };

                let mut command_proc = Command::new(&command);
                if !arg_list.is_empty() {
                    command_proc.args(&arg_list);
                }
                for (key, value) in &env_map {
                    command_proc.env(key, value);
                }
                if let Some(dir) = &cwd {
                    command_proc.current_dir(dir);
                }
                if stdin_data.is_some() {
                    command_proc.stdin(Stdio::piped());
                } else {
                    command_proc.stdin(Stdio::null());
                }
                command_proc.stdout(Stdio::piped());
                command_proc.stderr(Stdio::piped());

                let mut child = command_proc
                    .spawn()
                    .map_err(|error| VmError::Runtime(process_error("run", &command, &error)))?;

                if let Some(input) = stdin_data {
                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(input.as_bytes()).map_err(|error| {
                            VmError::Runtime(process_error("run", &command, &error))
                        })?;
                    }
                }

                let output = child
                    .wait_with_output()
                    .map_err(|error| VmError::Runtime(process_error("run", &command, &error)))?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1) as i64;
                let result = self.make_process_result(command.clone(), exit_code, stdout, stderr);
                self.stack.push(result);
            }
            StdFunctionKind::ProcessSpawn => {
                if args.is_empty() || args.len() > 4 {
                    bail!(VmError::Runtime(format!(
                        "process.spawn expected between 1 and 4 arguments but got {}",
                        args.len()
                    )));
                }
                let command = self
                    .expect_string(&args[0], "process.spawn expects the command to be a String")?;
                let arg_list = if args.len() >= 2 {
                    match &args[1] {
                        Value::Nil => Vec::new(),
                        value => self.expect_string_list(
                            value,
                            "process.spawn expects args to be a List[String]",
                        )?,
                    }
                } else {
                    Vec::new()
                };
                let env_map = if args.len() >= 3 {
                    match &args[2] {
                        Value::Nil => HashMap::new(),
                        value => self.expect_string_dict(
                            value,
                            "process.spawn expects env to be a Dict[String, String]",
                        )?,
                    }
                } else {
                    HashMap::new()
                };
                let cwd = if args.len() >= 4 {
                    match &args[3] {
                        Value::Nil => None,
                        value => Some(
                            self.expect_string(value, "process.spawn expects cwd to be a String")?,
                        ),
                    }
                } else {
                    None
                };

                let mut command_proc = Command::new(&command);
                if !arg_list.is_empty() {
                    command_proc.args(&arg_list);
                }
                for (key, value) in &env_map {
                    command_proc.env(key, value);
                }
                if let Some(dir) = &cwd {
                    command_proc.current_dir(dir);
                }
                command_proc.stdin(Stdio::piped());
                command_proc.stdout(Stdio::piped());
                command_proc.stderr(Stdio::piped());

                let mut child = command_proc
                    .spawn()
                    .map_err(|error| VmError::Runtime(process_error("spawn", &command, &error)))?;

                let stdout = child.stdout.take().map(BufReader::new);
                let stderr = child.stderr.take().map(BufReader::new);
                let stdin = child.stdin.take();

                let handle_id = self.next_process_handle;
                self.next_process_handle += 1;
                self.process_handles.insert(
                    handle_id,
                    ProcessEntry {
                        child,
                        stdout,
                        stderr,
                        stdin,
                        command: command.clone(),
                    },
                );
                self.stack.push(Value::Int(handle_id));
            }
            StdFunctionKind::ProcessReadStdout => {
                if args.is_empty() || args.len() > 2 {
                    bail!(VmError::Runtime(format!(
                        "process.read_stdout expected 1 or 2 arguments but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.read_stdout expects the handle to be an Int".to_string()
                        ))
                    }
                };
                let size = if args.len() == 2 {
                    match &args[1] {
                        Value::Nil => None,
                        Value::Int(value) if *value > 0 => Some(*value as usize),
                        Value::Int(_) => {
                            bail!(VmError::Runtime(
                                "process.read_stdout expects bytes to be a positive Int"
                                    .to_string()
                            ))
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "process.read_stdout expects bytes to be an Int".to_string()
                            ))
                        }
                    }
                } else {
                    None
                };
                let entry = self.process_handles.get_mut(&handle_id).ok_or_else(|| {
                    VmError::Runtime(format!("invalid process handle {handle_id}"))
                })?;
                let output = Self::read_from_pipe(&mut entry.stdout, size).map_err(|error| {
                    VmError::Runtime(process_error("read_stdout", &entry.command, &error))
                })?;
                self.stack.push(Value::String(output));
            }
            StdFunctionKind::ProcessReadStderr => {
                if args.is_empty() || args.len() > 2 {
                    bail!(VmError::Runtime(format!(
                        "process.read_stderr expected 1 or 2 arguments but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.read_stderr expects the handle to be an Int".to_string()
                        ))
                    }
                };
                let size = if args.len() == 2 {
                    match &args[1] {
                        Value::Nil => None,
                        Value::Int(value) if *value > 0 => Some(*value as usize),
                        Value::Int(_) => {
                            bail!(VmError::Runtime(
                                "process.read_stderr expects bytes to be a positive Int"
                                    .to_string()
                            ))
                        }
                        _ => {
                            bail!(VmError::Runtime(
                                "process.read_stderr expects bytes to be an Int".to_string()
                            ))
                        }
                    }
                } else {
                    None
                };
                let entry = self.process_handles.get_mut(&handle_id).ok_or_else(|| {
                    VmError::Runtime(format!("invalid process handle {handle_id}"))
                })?;
                let output = Self::read_from_pipe(&mut entry.stderr, size).map_err(|error| {
                    VmError::Runtime(process_error("read_stderr", &entry.command, &error))
                })?;
                self.stack.push(Value::String(output));
            }
            StdFunctionKind::ProcessWriteStdin => {
                if args.len() != 2 {
                    bail!(VmError::Runtime(format!(
                        "process.write_stdin expected 2 arguments but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.write_stdin expects the handle to be an Int".to_string()
                        ))
                    }
                };
                let data = self
                    .expect_string(&args[1], "process.write_stdin expects data to be a String")?;
                let entry = self.process_handles.get_mut(&handle_id).ok_or_else(|| {
                    VmError::Runtime(format!("invalid process handle {handle_id}"))
                })?;
                if let Some(stdin) = entry.stdin.as_mut() {
                    stdin.write_all(data.as_bytes()).map_err(|error| {
                        VmError::Runtime(process_error("write_stdin", &entry.command, &error))
                    })?;
                    self.stack.push(Value::Void);
                } else {
                    bail!(VmError::Runtime(
                        "process stdin has already been closed".to_string()
                    ));
                }
            }
            StdFunctionKind::ProcessCloseStdin => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "process.close_stdin expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.close_stdin expects the handle to be an Int".to_string()
                        ))
                    }
                };
                let entry = self.process_handles.get_mut(&handle_id).ok_or_else(|| {
                    VmError::Runtime(format!("invalid process handle {handle_id}"))
                })?;
                entry.stdin.take();
                self.stack.push(Value::Void);
            }
            StdFunctionKind::ProcessWait => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "process.wait expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.wait expects the handle to be an Int".to_string()
                        ))
                    }
                };
                let mut entry = self.process_handles.remove(&handle_id).ok_or_else(|| {
                    VmError::Runtime(format!("invalid process handle {handle_id}"))
                })?;
                let status = entry.child.wait().map_err(|error| {
                    VmError::Runtime(process_error("wait", &entry.command, &error))
                })?;
                let stdout = Self::read_from_pipe(&mut entry.stdout, None).map_err(|error| {
                    VmError::Runtime(process_error("wait", &entry.command, &error))
                })?;
                let stderr = Self::read_from_pipe(&mut entry.stderr, None).map_err(|error| {
                    VmError::Runtime(process_error("wait", &entry.command, &error))
                })?;
                entry.stdin.take();
                let exit_code = status.code().unwrap_or(-1) as i64;
                let result =
                    self.make_process_result(entry.command.clone(), exit_code, stdout, stderr);
                self.stack.push(result);
            }
            StdFunctionKind::ProcessKill => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "process.kill expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.kill expects the handle to be an Int".to_string()
                        ))
                    }
                };
                let entry = self.process_handles.get_mut(&handle_id).ok_or_else(|| {
                    VmError::Runtime(format!("invalid process handle {handle_id}"))
                })?;
                entry.child.kill().map_err(|error| {
                    VmError::Runtime(process_error("kill", &entry.command, &error))
                })?;
                self.stack.push(Value::Bool(true));
            }
            StdFunctionKind::ProcessClose => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "process.close expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let handle_id = match args[0] {
                    Value::Int(id) => id,
                    _ => {
                        bail!(VmError::Runtime(
                            "process.close expects the handle to be an Int".to_string()
                        ))
                    }
                };
                if let Some(mut entry) = self.process_handles.remove(&handle_id) {
                    let _ = entry.child.kill();
                }
                self.stack.push(Value::Void);
            }
            StdFunctionKind::CliArgs => {
                if !args.is_empty() {
                    bail!(VmError::Runtime(format!(
                        "cli.args expected 0 arguments but got {}",
                        args.len()
                    )));
                }
                let values: Vec<Value> = self.cli_args.iter().cloned().map(Value::String).collect();
                self.stack.push(Value::List(Rc::new(values)));
            }
            StdFunctionKind::CliParse => {
                if !(args.len() == 1 || args.len() == 2) {
                    bail!(VmError::Runtime(format!(
                        "cli.parse expected 1 or 2 arguments but got {}",
                        args.len()
                    )));
                }
                if !matches!(args[0], Value::Dict(_)) {
                    bail!(VmError::Runtime(
                        "cli.parse expects the first argument to be a Dict".to_string()
                    ));
                }
                let override_args = if args.len() == 2 {
                    Some(self.expect_string_list(
                        &args[1],
                        "cli.parse expects additional arguments to be a list of Strings",
                    )?)
                } else {
                    None
                };
                let cli_args = override_args.unwrap_or_else(|| self.cli_args.clone());
                let outcome = parse_cli(&args[0], &cli_args, self.program_name.as_deref())
                    .map_err(|error| VmError::Runtime(cli_error("parse", &error)))?;
                let value = self.make_cli_parse_result(&outcome);
                self.stack.push(value);
            }
            StdFunctionKind::CliCapture => {
                if args.len() != 1 {
                    bail!(VmError::Runtime(format!(
                        "cli.capture expected 1 argument but got {}",
                        args.len()
                    )));
                }
                let argv =
                    self.expect_string_list(&args[0], "cli.capture expects a list of Strings")?;
                if argv.is_empty() {
                    bail!(VmError::Runtime(
                        "cli.capture requires at least one argument".to_string()
                    ));
                }
                let mut command = Command::new(&argv[0]);
                if argv.len() > 1 {
                    command.args(&argv[1..]);
                }
                let output = command.output().map_err(|error| {
                    VmError::Runtime(cli_target_error("capture", &argv[0], &error))
                })?;
                let exit_code = output.status.code().unwrap_or(-1) as i64;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let value = self.make_cli_result(exit_code, stdout, stderr);
                self.stack.push(value);
            }
        }
        Ok(())
    }

    fn expect_string(&self, value: &Value, context: &str) -> Result<String> {
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

    fn dict_from_map(map: &HashMap<String, Value>) -> Value {
        Value::Dict(Rc::new(
            map.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ))
    }

    fn list_from_strings(items: &[String]) -> Value {
        Value::List(Rc::new(
            items.iter().cloned().map(Value::String).collect::<Vec<_>>(),
        ))
    }

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

    fn handle_snapshot_assertion(
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
