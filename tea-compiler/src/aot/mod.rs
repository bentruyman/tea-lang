#![cfg(feature = "llvm-aot")]

use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module as LlvmModule};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, FloatType, IntType, PointerType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, CallSiteValue, FloatValue, FunctionValue,
    GlobalValue, IntValue, PointerValue, StructValue,
};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

pub type OptimizationLevel = inkwell::OptimizationLevel;

use crate::ast::{
    BinaryExpression, BinaryOperator, CallExpression, CatchHandler, CatchKind, ConditionalKind,
    ConditionalStatement, Expression, ExpressionKind, FunctionStatement,
    InterpolatedStringExpression, InterpolatedStringPart, LambdaBody, LambdaExpression, Literal,
    LoopHeader, LoopKind, LoopStatement, MatchPattern, Module as AstModule, ReturnStatement,
    SourceSpan, Statement, ThrowStatement, TryExpression, TypeExpression, UseStatement,
    VarStatement,
};
use crate::resolver::{Resolver, ResolverOutput};
use crate::stdlib::{self, StdFunctionKind};
use crate::typechecker::{
    ErrorDefinition, FunctionInstance, StructDefinition, StructInstance, StructType, Type,
    TypeChecker,
};

fn format_type_name(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::String => "String".to_string(),
        Type::Nil => "Nil".to_string(),
        Type::Void => "Void".to_string(),
        Type::List(inner) => format!("List[{}]", format_type_name(inner)),
        Type::Dict(inner) => format!("Dict[String, {}]", format_type_name(inner)),
        Type::Optional(inner) => format!("{}?", format_type_name(inner)),
        Type::Function(params, return_type) => {
            let param_str = if params.is_empty() {
                "()".to_string()
            } else {
                let joined = params
                    .iter()
                    .map(|param| format_type_name(param))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({joined})")
            };
            format!("Func{param_str} -> {}", format_type_name(return_type))
        }
        Type::Struct(struct_type) => format_struct_type_name(struct_type),
        Type::Enum(enum_type) => enum_type.name.clone(),
        Type::Union(union_type) => union_type.name.clone(),
        Type::Error(error_type) => match &error_type.variant {
            Some(variant) => format!("{}.{}", error_type.name, variant),
            None => error_type.name.clone(),
        },
        Type::GenericParameter(name) => name.clone(),
        Type::Unknown => "Unknown".to_string(),
    }
}

fn format_struct_type_name(struct_type: &StructType) -> String {
    if struct_type.type_arguments.is_empty() {
        struct_type.name.clone()
    } else {
        let args = struct_type
            .type_arguments
            .iter()
            .map(|arg| format_type_name(arg))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}[{}]", struct_type.name, args)
    }
}

fn type_to_value_type(ty: &Type) -> Result<ValueType> {
    match ty {
        Type::Bool => Ok(ValueType::Bool),
        Type::Int => Ok(ValueType::Int),
        Type::Float => Ok(ValueType::Float),
        Type::String => Ok(ValueType::String),
        Type::Nil => Ok(ValueType::Void),
        Type::Void => Ok(ValueType::Void),
        Type::List(inner) => Ok(ValueType::List(Box::new(type_to_value_type(inner)?))),
        Type::Function(params, return_type) => {
            let mut lowered_params = Vec::with_capacity(params.len());
            for param in params {
                lowered_params.push(type_to_value_type(param)?);
            }
            Ok(ValueType::Function(
                lowered_params,
                Box::new(type_to_value_type(return_type)?),
            ))
        }
        Type::Struct(struct_type) => Ok(ValueType::Struct(format_struct_type_name(struct_type))),
        Type::Enum(enum_type) => bail!(format!(
            "LLVM backend does not yet support enums like '{}'",
            enum_type.name
        )),
        Type::Union(union_type) => bail!(format!(
            "LLVM backend does not yet support union '{}'",
            union_type.name
        )),
        Type::GenericParameter(name) => {
            bail!(format!(
                "cannot lower generic parameter '{}' in LLVM backend",
                name
            ))
        }
        Type::Dict(inner) => Ok(ValueType::Dict(Box::new(type_to_value_type(inner)?))),
        Type::Optional(inner) => Ok(ValueType::Optional(Box::new(type_to_value_type(inner)?))),
        Type::Error(error_type) => Ok(ValueType::Error {
            error_name: error_type.name.clone(),
            variant_name: error_type.variant.clone(),
        }),
        Type::Unknown => bail!("cannot lower Unknown type in LLVM backend"),
    }
}

fn sanitize_symbol_component(component: &str) -> String {
    component
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn mangle_function_name(name: &str, type_arguments: &[Type]) -> String {
    if type_arguments.is_empty() {
        return name.to_string();
    }
    let components: Vec<String> = type_arguments
        .iter()
        .map(|ty| sanitize_symbol_component(&format_type_name(ty)))
        .collect();
    format!("{}$g{}", name, components.join("$"))
}

struct SemanticMetadata {
    lambda_captures: HashMap<usize, Vec<String>>,
    lambda_signatures: HashMap<usize, LambdaSignature>,
    struct_definitions: HashMap<String, StructDefinition>,
    error_definitions: HashMap<String, ErrorDefinition>,
    function_instances: HashMap<String, Vec<FunctionInstance>>,
    struct_instances: HashMap<String, Vec<StructInstance>>,
    function_call_metadata: HashMap<SourceSpan, (String, FunctionInstance)>,
    struct_call_metadata: HashMap<SourceSpan, (String, StructInstance)>,
    binding_types: HashMap<SourceSpan, Type>,
    type_test_metadata: HashMap<SourceSpan, Type>,
}

fn collect_semantic_metadata(module_ast: &AstModule) -> Result<SemanticMetadata> {
    let mut resolver = Resolver::new();
    resolver.resolve_module(module_ast);
    let ResolverOutput {
        diagnostics: resolve_diagnostics,
        lambda_captures,
        ..
    } = resolver.into_parts();
    if resolve_diagnostics.has_errors() {
        bail!("Name resolution failed for LLVM lowering");
    }

    let mut type_checker = TypeChecker::new();
    type_checker.check_module(module_ast);
    let lambda_types = type_checker.lambda_types().clone();
    let struct_definitions = type_checker.struct_definitions();
    let error_definitions = type_checker.error_definitions();
    let function_instances = type_checker.function_instances().clone();
    let struct_instances = type_checker.struct_instances().clone();
    let function_call_metadata = type_checker.function_call_metadata().clone();
    let struct_call_metadata = type_checker.struct_call_metadata().clone();
    let binding_types = type_checker.binding_types().clone();
    let type_test_metadata = type_checker.type_test_metadata().clone();
    let type_diagnostics = type_checker.into_diagnostics();
    if type_diagnostics.has_errors() {
        bail!("Type checking failed for LLVM lowering");
    }

    let mut lambda_signatures = HashMap::new();
    for (id, ty) in lambda_types {
        let Type::Function(params, return_type) = ty else {
            bail!("lambda {id} did not resolve to a function type");
        };

        let mut lowered_params = Vec::with_capacity(params.len());
        for param in params {
            lowered_params.push(type_to_value_type(&param)?);
        }
        let lowered_return = type_to_value_type(&return_type)?;
        lambda_signatures.insert(
            id,
            LambdaSignature {
                param_types: lowered_params,
                return_type: lowered_return,
            },
        );
    }

    Ok(SemanticMetadata {
        lambda_captures,
        lambda_signatures,
        struct_definitions,
        error_definitions,
        function_instances,
        struct_instances,
        function_call_metadata,
        struct_call_metadata,
        binding_types,
        type_test_metadata,
    })
}

pub fn compile_module_to_llvm_ir(module_ast: &AstModule) -> Result<String> {
    let context = Context::create();
    let module = context.create_module("tea_module");
    let builder = context.create_builder();

    let metadata = collect_semantic_metadata(module_ast)?;
    let mut generator = LlvmCodeGenerator::new(&context, module, builder, metadata);
    generator.compile(module_ast)?;
    let module = generator.into_module();
    module
        .verify()
        .map_err(|e| anyhow!(format!("LLVM verification failed: {e}")))?;
    Ok(module.print_to_string().to_string())
}

pub struct ObjectCompileOptions<'a> {
    pub triple: Option<&'a str>,
    pub cpu: Option<&'a str>,
    pub features: Option<&'a str>,
    pub opt_level: OptimizationLevel,
    pub entry_symbol: Option<&'a str>,
}

impl<'a> Default for ObjectCompileOptions<'a> {
    fn default() -> Self {
        Self {
            triple: None,
            cpu: None,
            features: None,
            opt_level: OptimizationLevel::Default,
            entry_symbol: None,
        }
    }
}

pub fn compile_module_to_object(
    module_ast: &AstModule,
    output_path: &std::path::Path,
    options: &ObjectCompileOptions<'_>,
) -> Result<()> {
    let context = Context::create();
    let module = context.create_module("tea_module");
    let builder = context.create_builder();

    let metadata = collect_semantic_metadata(module_ast)?;
    let mut generator = LlvmCodeGenerator::new(&context, module, builder, metadata);
    generator.compile(module_ast)?;
    let module = generator.into_module();
    module
        .verify()
        .map_err(|e| anyhow!(format!("LLVM verification failed: {e}")))?;

    Target::initialize_all(&InitializationConfig::default());

    let raw_triple = options.triple.map(str::to_string).unwrap_or_else(|| {
        TargetMachine::get_default_triple()
            .as_str()
            .to_str()
            .unwrap_or("unknown-unknown-unknown")
            .to_string()
    });
    let triple_str = normalize_target_triple(&raw_triple);
    let target_triple = TargetTriple::create(&triple_str);
    module.set_triple(&target_triple);

    let target = Target::from_triple(&target_triple).map_err(|e| {
        anyhow!(format!(
            "failed to lookup target triple '{triple_str}': {e}"
        ))
    })?;

    let cpu = options.cpu.unwrap_or("generic");
    let features = options.features.unwrap_or("");
    let opt_level = options.opt_level;

    let reloc_mode = if triple_str.contains("windows") {
        RelocMode::Default
    } else {
        RelocMode::PIC
    };

    let target_machine = target
        .create_target_machine(
            &target_triple,
            cpu,
            features,
            opt_level,
            reloc_mode,
            CodeModel::Default,
        )
        .ok_or_else(|| anyhow!("failed to create target machine"))?;

    let data_layout = target_machine.get_target_data().get_data_layout();
    module.set_data_layout(&data_layout);

    if let Some(symbol) = options.entry_symbol {
        if let Some(main_fn) = module.get_function("main") {
            main_fn.as_global_value().set_name(symbol);
        }
    }

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    target_machine
        .write_to_file(&module, FileType::Object, output_path)
        .map_err(|e| anyhow!(format!("failed to write object file: {e}")))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Int,
    Float,
    Bool,
    String,
    List(Box<ValueType>),
    Dict(Box<ValueType>),
    Function(Vec<ValueType>, Box<ValueType>),
    Struct(String),
    Error {
        error_name: String,
        variant_name: Option<String>,
    },
    Optional(Box<ValueType>),
    Void,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ErrorHandlingMode {
    Propagate,
    Capture,
}

#[derive(Clone)]
struct LambdaSignature {
    param_types: Vec<ValueType>,
    return_type: ValueType,
}

#[derive(Clone)]
struct FunctionSignature<'ctx> {
    value: FunctionValue<'ctx>,
    return_type: ValueType,
    param_types: Vec<ValueType>,
    can_throw: bool,
}

#[derive(Clone)]
struct LocalVariable<'ctx> {
    pointer: PointerValue<'ctx>,
    ty: ValueType,
    mutable: bool,
}

struct StructLowering<'ctx> {
    field_names: Vec<String>,
    field_types: Vec<ValueType>,
    template_global: Option<GlobalValue<'ctx>>,
    field_names_global: Option<GlobalValue<'ctx>>,
    template_pointer: Option<PointerValue<'ctx>>,
}

impl<'ctx> StructLowering<'ctx> {
    fn new() -> Self {
        Self {
            field_names: Vec::new(),
            field_types: Vec::new(),
            template_global: None,
            field_names_global: None,
            template_pointer: None,
        }
    }

    fn field_index(&self, name: &str) -> Option<usize> {
        self.field_names.iter().position(|field| field == name)
    }
}

#[derive(Clone)]
struct ErrorVariantLowering<'ctx> {
    field_names: Vec<String>,
    field_types: Vec<ValueType>,
    template_global: Option<GlobalValue<'ctx>>,
    field_names_global: Option<GlobalValue<'ctx>>,
    template_pointer: Option<PointerValue<'ctx>>,
}

impl<'ctx> ErrorVariantLowering<'ctx> {
    fn new() -> Self {
        Self {
            field_names: Vec::new(),
            field_types: Vec::new(),
            template_global: None,
            field_names_global: None,
            template_pointer: None,
        }
    }
}

#[derive(Clone)]
struct GlobalBindingSlot<'ctx> {
    pointer: GlobalValue<'ctx>,
    ty: ValueType,
    mutable: bool,
    initialized: bool,
}

#[derive(Clone)]
enum ExprValue<'ctx> {
    Int(IntValue<'ctx>),
    Float(FloatValue<'ctx>),
    Bool(IntValue<'ctx>),
    String(PointerValue<'ctx>),
    List {
        pointer: PointerValue<'ctx>,
        element_type: Box<ValueType>,
    },
    Dict {
        pointer: PointerValue<'ctx>,
        value_type: Box<ValueType>,
    },
    Struct {
        pointer: PointerValue<'ctx>,
        struct_name: String,
    },
    Error {
        pointer: PointerValue<'ctx>,
        error_name: String,
        variant_name: Option<String>,
    },
    Closure {
        pointer: PointerValue<'ctx>,
        param_types: Vec<ValueType>,
        return_type: Box<ValueType>,
    },
    Optional {
        value: StructValue<'ctx>,
        inner: Box<ValueType>,
    },
    Void,
}

impl<'ctx> ExprValue<'ctx> {
    fn ty(&self) -> ValueType {
        match self {
            ExprValue::Int(_) => ValueType::Int,
            ExprValue::Float(_) => ValueType::Float,
            ExprValue::Bool(_) => ValueType::Bool,
            ExprValue::String(_) => ValueType::String,
            ExprValue::List { element_type, .. } => ValueType::List(element_type.clone()),
            ExprValue::Dict { value_type, .. } => ValueType::Dict(value_type.clone()),
            ExprValue::Struct { struct_name, .. } => ValueType::Struct(struct_name.clone()),
            ExprValue::Error {
                error_name,
                variant_name,
                ..
            } => ValueType::Error {
                error_name: error_name.clone(),
                variant_name: variant_name.clone(),
            },
            ExprValue::Closure {
                param_types,
                return_type,
                ..
            } => ValueType::Function(param_types.clone(), return_type.clone()),
            ExprValue::Optional { inner, .. } => ValueType::Optional(inner.clone()),
            ExprValue::Void => ValueType::Void,
        }
    }

    fn into_basic_value(self) -> Option<BasicValueEnum<'ctx>> {
        match self {
            ExprValue::Int(v) => Some(v.into()),
            ExprValue::Float(v) => Some(v.into()),
            ExprValue::Bool(v) => Some(v.into()),
            ExprValue::String(ptr) => Some(ptr.into()),
            ExprValue::List { pointer, .. } => Some(pointer.into()),
            ExprValue::Dict { pointer, .. } => Some(pointer.into()),
            ExprValue::Struct { pointer, .. } => Some(pointer.into()),
            ExprValue::Error { pointer, .. } => Some(pointer.into()),
            ExprValue::Closure { pointer, .. } => Some(pointer.into()),
            ExprValue::Optional { value, .. } => Some(value.into()),
            ExprValue::Void => None,
        }
    }

    fn into_int(self) -> Result<IntValue<'ctx>> {
        match self {
            ExprValue::Int(v) => Ok(v),
            _ => bail!("expected Int value"),
        }
    }

    fn into_bool(self) -> Result<IntValue<'ctx>> {
        match self {
            ExprValue::Bool(v) => Ok(v),
            _ => bail!("expected Bool value"),
        }
    }

    fn into_string(self) -> Result<PointerValue<'ctx>> {
        match self {
            ExprValue::String(ptr) => Ok(ptr),
            _ => bail!("expected String value"),
        }
    }
}

struct LlvmCodeGenerator<'ctx> {
    context: &'ctx Context,
    module: LlvmModule<'ctx>,
    builder: Builder<'ctx>,
    functions: HashMap<String, FunctionSignature<'ctx>>,
    ptr_type: PointerType<'ctx>,
    tea_value: inkwell::types::StructType<'ctx>,
    tea_struct_template: inkwell::types::StructType<'ctx>,
    tea_closure: inkwell::types::StructType<'ctx>,
    tea_error_template: inkwell::types::StructType<'ctx>,
    string_counter: usize,
    structs: HashMap<String, StructLowering<'ctx>>,
    errors: HashMap<String, HashMap<String, ErrorVariantLowering<'ctx>>>,
    lambda_captures: HashMap<usize, Vec<String>>,
    lambda_signatures: HashMap<usize, LambdaSignature>,
    function_instances_tc: HashMap<String, Vec<FunctionInstance>>,
    struct_instances_tc: HashMap<String, Vec<StructInstance>>,
    function_call_metadata_tc: HashMap<SourceSpan, (String, FunctionInstance)>,
    struct_call_metadata_tc: HashMap<SourceSpan, (String, StructInstance)>,
    binding_types_tc: HashMap<SourceSpan, Type>,
    type_test_metadata_tc: HashMap<SourceSpan, Type>,
    global_slots: HashMap<String, GlobalBindingSlot<'ctx>>,
    struct_field_variants: HashMap<String, Vec<ValueType>>,
    struct_variant_bases: HashMap<String, String>,
    struct_definitions_tc: HashMap<String, StructDefinition>,
    error_definitions_tc: HashMap<String, ErrorDefinition>,
    generic_binding_stack: Vec<HashMap<String, (Type, ValueType)>>,
    lambda_functions: HashMap<usize, FunctionValue<'ctx>>,
    lambda_capture_types: HashMap<usize, Vec<ValueType>>,
    builtin_functions: HashMap<String, StdFunctionKind>,
    module_builtins: HashMap<String, HashMap<String, StdFunctionKind>>,
    builtin_print_int: Option<FunctionValue<'ctx>>,
    builtin_print_float: Option<FunctionValue<'ctx>>,
    builtin_print_bool: Option<FunctionValue<'ctx>>,
    builtin_print_string: Option<FunctionValue<'ctx>>,
    builtin_print_list: Option<FunctionValue<'ctx>>,
    builtin_print_dict: Option<FunctionValue<'ctx>>,
    builtin_print_closure: Option<FunctionValue<'ctx>>,
    builtin_print_struct: Option<FunctionValue<'ctx>>,
    builtin_print_error: Option<FunctionValue<'ctx>>,
    builtin_assert_fn: Option<FunctionValue<'ctx>>,
    builtin_assert_eq_fn: Option<FunctionValue<'ctx>>,
    builtin_assert_ne_fn: Option<FunctionValue<'ctx>>,
    builtin_fail_fn: Option<FunctionValue<'ctx>>,
    util_len_fn: Option<FunctionValue<'ctx>>,
    util_to_string_fn: Option<FunctionValue<'ctx>>,
    string_concat_fn: Option<FunctionValue<'ctx>>,
    util_clamp_int_fn: Option<FunctionValue<'ctx>>,
    util_is_nil_fn: Option<FunctionValue<'ctx>>,
    util_is_bool_fn: Option<FunctionValue<'ctx>>,
    util_is_int_fn: Option<FunctionValue<'ctx>>,
    util_is_float_fn: Option<FunctionValue<'ctx>>,
    util_is_string_fn: Option<FunctionValue<'ctx>>,
    util_is_list_fn: Option<FunctionValue<'ctx>>,
    util_is_struct_fn: Option<FunctionValue<'ctx>>,
    util_is_error_fn: Option<FunctionValue<'ctx>>,
    env_get_fn: Option<FunctionValue<'ctx>>,
    env_get_or_fn: Option<FunctionValue<'ctx>>,
    env_has_fn: Option<FunctionValue<'ctx>>,
    env_require_fn: Option<FunctionValue<'ctx>>,
    env_set_fn: Option<FunctionValue<'ctx>>,
    env_unset_fn: Option<FunctionValue<'ctx>>,
    env_vars_fn: Option<FunctionValue<'ctx>>,
    env_cwd_fn: Option<FunctionValue<'ctx>>,
    env_set_cwd_fn: Option<FunctionValue<'ctx>>,
    env_temp_dir_fn: Option<FunctionValue<'ctx>>,
    env_home_dir_fn: Option<FunctionValue<'ctx>>,
    env_config_dir_fn: Option<FunctionValue<'ctx>>,
    path_join_fn: Option<FunctionValue<'ctx>>,
    path_components_fn: Option<FunctionValue<'ctx>>,
    path_dirname_fn: Option<FunctionValue<'ctx>>,
    path_basename_fn: Option<FunctionValue<'ctx>>,
    path_extension_fn: Option<FunctionValue<'ctx>>,
    path_set_extension_fn: Option<FunctionValue<'ctx>>,
    path_strip_extension_fn: Option<FunctionValue<'ctx>>,
    path_normalize_fn: Option<FunctionValue<'ctx>>,
    path_absolute_fn: Option<FunctionValue<'ctx>>,
    path_relative_fn: Option<FunctionValue<'ctx>>,
    path_is_absolute_fn: Option<FunctionValue<'ctx>>,
    path_separator_fn: Option<FunctionValue<'ctx>>,
    cli_args_fn: Option<FunctionValue<'ctx>>,
    cli_parse_fn: Option<FunctionValue<'ctx>>,
    process_run_fn: Option<FunctionValue<'ctx>>,
    process_spawn_fn: Option<FunctionValue<'ctx>>,
    process_wait_fn: Option<FunctionValue<'ctx>>,
    process_kill_fn: Option<FunctionValue<'ctx>>,
    process_read_stdout_fn: Option<FunctionValue<'ctx>>,
    process_read_stderr_fn: Option<FunctionValue<'ctx>>,
    process_write_stdin_fn: Option<FunctionValue<'ctx>>,
    process_close_stdin_fn: Option<FunctionValue<'ctx>>,
    process_close_fn: Option<FunctionValue<'ctx>>,
    fs_read_text_fn: Option<FunctionValue<'ctx>>,
    fs_write_text_fn: Option<FunctionValue<'ctx>>,
    fs_write_text_atomic_fn: Option<FunctionValue<'ctx>>,
    fs_read_bytes_fn: Option<FunctionValue<'ctx>>,
    fs_write_bytes_fn: Option<FunctionValue<'ctx>>,
    fs_write_bytes_atomic_fn: Option<FunctionValue<'ctx>>,
    fs_create_dir_fn: Option<FunctionValue<'ctx>>,
    fs_ensure_dir_fn: Option<FunctionValue<'ctx>>,
    fs_ensure_parent_fn: Option<FunctionValue<'ctx>>,
    fs_remove_fn: Option<FunctionValue<'ctx>>,
    fs_exists_fn: Option<FunctionValue<'ctx>>,
    fs_is_dir_fn: Option<FunctionValue<'ctx>>,
    fs_is_symlink_fn: Option<FunctionValue<'ctx>>,
    fs_list_dir_fn: Option<FunctionValue<'ctx>>,
    fs_walk_fn: Option<FunctionValue<'ctx>>,
    fs_glob_fn: Option<FunctionValue<'ctx>>,
    fs_size_fn: Option<FunctionValue<'ctx>>,
    fs_modified_fn: Option<FunctionValue<'ctx>>,
    fs_permissions_fn: Option<FunctionValue<'ctx>>,
    fs_is_readonly_fn: Option<FunctionValue<'ctx>>,
    fs_metadata_fn: Option<FunctionValue<'ctx>>,
    fs_open_read_fn: Option<FunctionValue<'ctx>>,
    fs_read_chunk_fn: Option<FunctionValue<'ctx>>,
    fs_close_fn: Option<FunctionValue<'ctx>>,
    alloc_string_fn: Option<FunctionValue<'ctx>>,
    alloc_list_fn: Option<FunctionValue<'ctx>>,
    alloc_struct_fn: Option<FunctionValue<'ctx>>,
    list_set_fn: Option<FunctionValue<'ctx>>,
    list_get_fn: Option<FunctionValue<'ctx>>,
    struct_set_fn: Option<FunctionValue<'ctx>>,
    struct_get_fn: Option<FunctionValue<'ctx>>,
    error_alloc_fn: Option<FunctionValue<'ctx>>,
    error_set_fn: Option<FunctionValue<'ctx>>,
    error_get_fn: Option<FunctionValue<'ctx>>,
    error_current_fn: Option<FunctionValue<'ctx>>,
    error_set_current_fn: Option<FunctionValue<'ctx>>,
    error_clear_current_fn: Option<FunctionValue<'ctx>>,
    error_get_template_fn: Option<FunctionValue<'ctx>>,
    value_from_int_fn: Option<FunctionValue<'ctx>>,
    value_from_float_fn: Option<FunctionValue<'ctx>>,
    value_from_bool_fn: Option<FunctionValue<'ctx>>,
    value_from_string_fn: Option<FunctionValue<'ctx>>,
    value_from_list_fn: Option<FunctionValue<'ctx>>,
    value_from_dict_fn: Option<FunctionValue<'ctx>>,
    value_from_struct_fn: Option<FunctionValue<'ctx>>,
    value_from_error_fn: Option<FunctionValue<'ctx>>,
    value_from_closure_fn: Option<FunctionValue<'ctx>>,
    value_nil_fn: Option<FunctionValue<'ctx>>,
    value_as_int_fn: Option<FunctionValue<'ctx>>,
    value_as_float_fn: Option<FunctionValue<'ctx>>,
    value_as_bool_fn: Option<FunctionValue<'ctx>>,
    value_as_string_fn: Option<FunctionValue<'ctx>>,
    value_as_list_fn: Option<FunctionValue<'ctx>>,
    value_as_dict_fn: Option<FunctionValue<'ctx>>,
    value_as_struct_fn: Option<FunctionValue<'ctx>>,
    value_as_error_fn: Option<FunctionValue<'ctx>>,
    value_as_closure_fn: Option<FunctionValue<'ctx>>,
    string_equal_fn: Option<FunctionValue<'ctx>>,
    list_equal_fn: Option<FunctionValue<'ctx>>,
    dict_new_fn: Option<FunctionValue<'ctx>>,
    dict_set_fn: Option<FunctionValue<'ctx>>,
    dict_get_fn: Option<FunctionValue<'ctx>>,
    dict_equal_fn: Option<FunctionValue<'ctx>>,
    struct_equal_fn: Option<FunctionValue<'ctx>>,
    closure_new_fn: Option<FunctionValue<'ctx>>,
    closure_set_fn: Option<FunctionValue<'ctx>>,
    closure_get_fn: Option<FunctionValue<'ctx>>,
    closure_equal_fn: Option<FunctionValue<'ctx>>,
    io_read_line_fn: Option<FunctionValue<'ctx>>,
    io_read_all_fn: Option<FunctionValue<'ctx>>,
    io_read_bytes_fn: Option<FunctionValue<'ctx>>,
    io_write_fn: Option<FunctionValue<'ctx>>,
    io_write_err_fn: Option<FunctionValue<'ctx>>,
    io_flush_fn: Option<FunctionValue<'ctx>>,
    json_encode_fn: Option<FunctionValue<'ctx>>,
    json_decode_fn: Option<FunctionValue<'ctx>>,
    yaml_encode_fn: Option<FunctionValue<'ctx>>,
    yaml_decode_fn: Option<FunctionValue<'ctx>>,
    error_mode_stack: Vec<ErrorHandlingMode>,
    function_return_stack: Vec<ValueType>,
}

impl<'ctx> LlvmCodeGenerator<'ctx> {
    fn new(
        context: &'ctx Context,
        module: LlvmModule<'ctx>,
        builder: Builder<'ctx>,
        metadata: SemanticMetadata,
    ) -> Self {
        let SemanticMetadata {
            lambda_captures,
            lambda_signatures,
            struct_definitions,
            error_definitions,
            function_instances,
            struct_instances,
            function_call_metadata,
            struct_call_metadata,
            binding_types,
            type_test_metadata,
        } = metadata;

        let tea_string = context.opaque_struct_type("TeaString");
        let tea_list = context.opaque_struct_type("TeaList");
        let tea_value = context.opaque_struct_type("TeaValue");
        let tea_struct_template = context.opaque_struct_type("TeaStructTemplate");
        let tea_struct_instance = context.opaque_struct_type("TeaStructInstance");
        let tea_error_template = context.opaque_struct_type("TeaErrorTemplate");
        let tea_closure = context.opaque_struct_type("TeaClosure");

        let i64 = context.i64_type();
        let ptr_type = context.ptr_type(AddressSpace::default());
        tea_string.set_body(&[i64.into(), ptr_type.into()], false);

        let value_payload = context.i64_type();
        tea_value.set_body(&[context.i32_type().into(), value_payload.into()], false);

        tea_list.set_body(&[i64.into(), i64.into(), ptr_type.into()], false);
        tea_struct_template.set_body(&[ptr_type.into(), i64.into(), ptr_type.into()], false);
        tea_struct_instance.set_body(&[ptr_type.into(), ptr_type.into()], false);
        tea_error_template.set_body(
            &[
                ptr_type.into(),
                ptr_type.into(),
                i64.into(),
                ptr_type.into(),
            ],
            false,
        );
        tea_closure.set_body(&[ptr_type.into(), ptr_type.into(), i64.into()], false);

        Self {
            context,
            module,
            builder,
            functions: HashMap::new(),
            ptr_type,
            tea_value,
            tea_struct_template,
            tea_error_template,
            tea_closure,
            string_counter: 0,
            structs: HashMap::new(),
            errors: HashMap::new(),
            lambda_captures,
            lambda_signatures,
            function_instances_tc: function_instances,
            struct_instances_tc: struct_instances,
            function_call_metadata_tc: function_call_metadata,
            struct_call_metadata_tc: struct_call_metadata,
            binding_types_tc: binding_types,
            type_test_metadata_tc: type_test_metadata,
            struct_field_variants: HashMap::new(),
            struct_variant_bases: HashMap::new(),
            struct_definitions_tc: struct_definitions,
            error_definitions_tc: error_definitions,
            generic_binding_stack: Vec::new(),
            lambda_functions: HashMap::new(),
            lambda_capture_types: HashMap::new(),
            builtin_functions: HashMap::new(),
            module_builtins: HashMap::new(),
            builtin_print_int: None,
            builtin_print_float: None,
            builtin_print_bool: None,
            builtin_print_string: None,
            builtin_print_list: None,
            builtin_print_dict: None,
            builtin_print_closure: None,
            builtin_print_struct: None,
            builtin_print_error: None,
            builtin_assert_fn: None,
            builtin_assert_eq_fn: None,
            builtin_assert_ne_fn: None,
            builtin_fail_fn: None,
            util_len_fn: None,
            util_to_string_fn: None,
            string_concat_fn: None,
            util_clamp_int_fn: None,
            util_is_nil_fn: None,
            util_is_bool_fn: None,
            util_is_int_fn: None,
            util_is_float_fn: None,
            util_is_string_fn: None,
            util_is_list_fn: None,
            util_is_struct_fn: None,
            util_is_error_fn: None,
            env_get_fn: None,
            env_get_or_fn: None,
            env_has_fn: None,
            env_require_fn: None,
            env_set_fn: None,
            env_unset_fn: None,
            env_vars_fn: None,
            env_cwd_fn: None,
            env_set_cwd_fn: None,
            env_temp_dir_fn: None,
            env_home_dir_fn: None,
            env_config_dir_fn: None,
            path_join_fn: None,
            path_components_fn: None,
            path_dirname_fn: None,
            path_basename_fn: None,
            path_extension_fn: None,
            path_set_extension_fn: None,
            path_strip_extension_fn: None,
            path_normalize_fn: None,
            path_absolute_fn: None,
            path_relative_fn: None,
            path_is_absolute_fn: None,
            path_separator_fn: None,
            cli_args_fn: None,
            cli_parse_fn: None,
            process_run_fn: None,
            process_spawn_fn: None,
            process_wait_fn: None,
            process_kill_fn: None,
            process_read_stdout_fn: None,
            process_read_stderr_fn: None,
            process_write_stdin_fn: None,
            process_close_stdin_fn: None,
            process_close_fn: None,
            fs_read_text_fn: None,
            fs_write_text_fn: None,
            fs_write_text_atomic_fn: None,
            fs_read_bytes_fn: None,
            fs_write_bytes_fn: None,
            fs_write_bytes_atomic_fn: None,
            fs_create_dir_fn: None,
            fs_ensure_dir_fn: None,
            fs_ensure_parent_fn: None,
            fs_remove_fn: None,
            fs_exists_fn: None,
            fs_is_dir_fn: None,
            fs_is_symlink_fn: None,
            fs_list_dir_fn: None,
            fs_walk_fn: None,
            fs_glob_fn: None,
            fs_size_fn: None,
            fs_modified_fn: None,
            fs_permissions_fn: None,
            fs_is_readonly_fn: None,
            fs_metadata_fn: None,
            fs_open_read_fn: None,
            fs_read_chunk_fn: None,
            fs_close_fn: None,
            alloc_string_fn: None,
            alloc_list_fn: None,
            alloc_struct_fn: None,
            list_set_fn: None,
            list_get_fn: None,
            struct_set_fn: None,
            struct_get_fn: None,
            error_alloc_fn: None,
            error_set_fn: None,
            error_get_fn: None,
            value_from_int_fn: None,
            value_from_float_fn: None,
            value_from_bool_fn: None,
            value_from_string_fn: None,
            value_from_list_fn: None,
            value_from_dict_fn: None,
            value_from_struct_fn: None,
            value_from_error_fn: None,
            value_from_closure_fn: None,
            value_nil_fn: None,
            value_as_int_fn: None,
            value_as_float_fn: None,
            value_as_bool_fn: None,
            value_as_string_fn: None,
            value_as_list_fn: None,
            value_as_dict_fn: None,
            value_as_struct_fn: None,
            value_as_error_fn: None,
            value_as_closure_fn: None,
            string_equal_fn: None,
            list_equal_fn: None,
            dict_new_fn: None,
            dict_set_fn: None,
            dict_get_fn: None,
            dict_equal_fn: None,
            struct_equal_fn: None,
            closure_new_fn: None,
            closure_set_fn: None,
            closure_get_fn: None,
            closure_equal_fn: None,
            io_read_line_fn: None,
            io_read_all_fn: None,
            io_read_bytes_fn: None,
            io_write_fn: None,
            io_write_err_fn: None,
            io_flush_fn: None,
            json_encode_fn: None,
            json_decode_fn: None,
            yaml_encode_fn: None,
            yaml_decode_fn: None,
            error_current_fn: None,
            error_set_current_fn: None,
            error_clear_current_fn: None,
            error_get_template_fn: None,
            error_mode_stack: vec![ErrorHandlingMode::Propagate],
            function_return_stack: Vec::new(),
            global_slots: HashMap::new(),
        }
    }

    fn into_module(self) -> LlvmModule<'ctx> {
        self.module
    }

    fn string_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn list_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn struct_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn struct_template_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn dict_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn error_template_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn error_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn closure_ptr_type(&self) -> PointerType<'ctx> {
        self.ptr_type
    }

    fn value_type(&self) -> inkwell::types::StructType<'ctx> {
        self.tea_value
    }

    fn current_error_mode(&self) -> ErrorHandlingMode {
        *self
            .error_mode_stack
            .last()
            .expect("error mode stack should not be empty")
    }

    fn push_error_mode(&mut self, mode: ErrorHandlingMode) {
        self.error_mode_stack.push(mode);
    }

    fn pop_error_mode(&mut self) {
        self.error_mode_stack
            .pop()
            .expect("error mode stack underflow");
    }

    fn push_function_return(&mut self, ty: ValueType) {
        self.function_return_stack.push(ty);
    }

    fn pop_function_return(&mut self) {
        self.function_return_stack
            .pop()
            .expect("function return stack underflow");
    }

    fn current_function_return_type(&self) -> &ValueType {
        self.function_return_stack
            .last()
            .expect("function return stack should not be empty")
    }

    fn pointer_equals(
        &mut self,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>> {
        let lhs = map_builder_error(self.builder.build_ptr_to_int(
            left,
            self.int_type(),
            &format!("{name}_lhs"),
        ))?;
        let rhs = map_builder_error(self.builder.build_ptr_to_int(
            right,
            self.int_type(),
            &format!("{name}_rhs"),
        ))?;
        map_builder_error(
            self.builder
                .build_int_compare(IntPredicate::EQ, lhs, rhs, name),
        )
    }

    fn handle_possible_error(&mut self, function: FunctionValue<'ctx>) -> Result<()> {
        if matches!(self.current_error_mode(), ErrorHandlingMode::Capture) {
            return Ok(());
        }

        let error_fn = self.ensure_error_current();
        let call = self.call_function(error_fn, &[], "error_current")?;
        let pointer = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_error_current returned no value"))?
            .into_pointer_value();
        let is_null = map_builder_error(self.builder.build_is_null(pointer, "error_is_null"))?;
        let success_block = self.context.append_basic_block(function, "error_ok");
        let error_block = self.context.append_basic_block(function, "error_propagate");
        map_builder_error(self.builder.build_conditional_branch(
            is_null,
            success_block,
            error_block,
        ))?;

        self.builder.position_at_end(error_block);
        let return_type = self.current_function_return_type().clone();
        self.emit_error_return(function, &return_type)?;

        self.builder.position_at_end(success_block);
        Ok(())
    }

    fn clear_error_state(&mut self) -> Result<()> {
        let clear_fn = self.ensure_error_clear_current();
        self.call_function(clear_fn, &[], "error_clear")?;
        Ok(())
    }

    fn call_function(
        &mut self,
        func: FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        name: &str,
    ) -> Result<CallSiteValue<'ctx>> {
        map_builder_error(self.builder.build_call(func, args, name))
    }

    fn compile(&mut self, module_ast: &AstModule) -> Result<()> {
        for statement in &module_ast.statements {
            if let Statement::Use(use_stmt) = statement {
                self.register_use(use_stmt)?;
            }
        }
        self.collect_structs(&module_ast.statements)?;
        self.collect_globals(&module_ast.statements)?;
        for statement in &module_ast.statements {
            if let Statement::Function(func) = statement {
                self.declare_function(func)?;
            }
        }
        for statement in &module_ast.statements {
            if let Statement::Function(func) = statement {
                self.compile_function_variants(func)?;
            }
        }
        self.compile_main(&module_ast.statements)?;
        Ok(())
    }

    fn register_use(&mut self, use_stmt: &UseStatement) -> Result<()> {
        let module_path = use_stmt.module_path.as_str();
        if let Some(module) = stdlib::find_module(module_path) {
            let entry = self
                .module_builtins
                .entry(use_stmt.alias.name.clone())
                .or_insert_with(HashMap::new);
            for function in module.functions {
                entry.insert(function.name.to_string(), function.kind);
            }
            Ok(())
        } else {
            bail!(format!("unknown module '{module_path}'"));
        }
    }

    fn ensure_struct_variant_metadata(&mut self, struct_type: &StructType) -> Result<()> {
        let variant_name = format_struct_type_name(struct_type);
        if self.struct_field_variants.contains_key(&variant_name) {
            self.struct_variant_bases
                .entry(variant_name.clone())
                .or_insert_with(|| struct_type.name.clone());
            return Ok(());
        }

        if let Some(instances) = self.struct_instances_tc.get(&struct_type.name) {
            if let Some(instance) = instances
                .iter()
                .find(|inst| inst.type_arguments == struct_type.type_arguments)
            {
                let mut field_types = Vec::with_capacity(instance.field_types.len());
                for ty in &instance.field_types {
                    field_types.push(type_to_value_type(ty)?);
                }
                self.struct_field_variants
                    .insert(variant_name.clone(), field_types);
                self.struct_variant_bases
                    .insert(variant_name, struct_type.name.clone());
                return Ok(());
            }
        }

        if let Some(info) = self.structs.get(&struct_type.name) {
            if !info.field_types.is_empty() {
                self.struct_field_variants
                    .insert(variant_name.clone(), info.field_types.clone());
                self.struct_variant_bases
                    .insert(variant_name, struct_type.name.clone());
                return Ok(());
            }
        }

        if let Ok(field_types) = self.lower_struct_fields_from_definition(struct_type) {
            self.struct_field_variants
                .insert(variant_name.clone(), field_types);
            self.struct_variant_bases
                .insert(variant_name, struct_type.name.clone());
            return Ok(());
        }

        if let Some((_, instance)) = self.struct_call_metadata_tc.values().find(|(base, inst)| {
            *base == struct_type.name && inst.type_arguments == struct_type.type_arguments
        }) {
            let mut field_types = Vec::with_capacity(instance.field_types.len());
            for ty in &instance.field_types {
                field_types.push(type_to_value_type(ty)?);
            }
            self.struct_field_variants
                .insert(variant_name.clone(), field_types);
            self.struct_variant_bases
                .insert(variant_name, struct_type.name.clone());
            return Ok(());
        }

        bail!(format!(
            "missing struct metadata for '{}'",
            struct_type.name
        ))
    }

    fn lower_struct_fields_from_definition(
        &self,
        struct_type: &StructType,
    ) -> Result<Vec<ValueType>> {
        let definition = self
            .struct_definitions_tc
            .get(&struct_type.name)
            .ok_or_else(|| anyhow!(format!("unknown struct '{}' in metadata", struct_type.name)))?;

        if definition.type_parameters.len() != struct_type.type_arguments.len() {
            bail!(format!(
                "struct '{}' expected {} type arguments, found {}",
                struct_type.name,
                definition.type_parameters.len(),
                struct_type.type_arguments.len()
            ));
        }

        let mut mapping: HashMap<String, Type> = HashMap::new();
        for (param, arg) in definition
            .type_parameters
            .iter()
            .cloned()
            .zip(struct_type.type_arguments.iter().cloned())
        {
            mapping.insert(param, arg);
        }

        let mut lowered = Vec::with_capacity(definition.fields.len());
        for field in &definition.fields {
            let substituted = self.substitute_type(&field.ty, &mapping)?;
            lowered.push(type_to_value_type(&substituted)?);
        }
        Ok(lowered)
    }

    fn substitute_type(&self, ty: &Type, mapping: &HashMap<String, Type>) -> Result<Type> {
        match ty {
            Type::GenericParameter(name) => Ok(mapping.get(name).cloned().unwrap_or(Type::Unknown)),
            Type::List(inner) => Ok(Type::List(Box::new(self.substitute_type(inner, mapping)?))),
            Type::Dict(inner) => Ok(Type::Dict(Box::new(self.substitute_type(inner, mapping)?))),
            Type::Function(params, return_type) => {
                let mut substituted_params = Vec::with_capacity(params.len());
                for param in params {
                    substituted_params.push(self.substitute_type(param, mapping)?);
                }
                let substituted_return = self.substitute_type(return_type, mapping)?;
                Ok(Type::Function(
                    substituted_params,
                    Box::new(substituted_return),
                ))
            }
            Type::Struct(inner_struct) => {
                let mut args = Vec::with_capacity(inner_struct.type_arguments.len());
                for arg in &inner_struct.type_arguments {
                    args.push(self.substitute_type(arg, mapping)?);
                }
                Ok(Type::Struct(StructType {
                    name: inner_struct.name.clone(),
                    type_arguments: args,
                }))
            }
            other => Ok(other.clone()),
        }
    }

    fn resolve_type_with_bindings(&self, ty: &Type) -> Result<Type> {
        match ty {
            Type::GenericParameter(name) => self
                .lookup_generic_type(name)
                .ok_or_else(|| anyhow!(format!("unbound generic parameter '{}'", name))),
            Type::List(inner) => Ok(Type::List(Box::new(
                self.resolve_type_with_bindings(inner)?,
            ))),
            Type::Dict(inner) => Ok(Type::Dict(Box::new(
                self.resolve_type_with_bindings(inner)?,
            ))),
            Type::Function(params, return_type) => {
                let mut resolved_params = Vec::with_capacity(params.len());
                for param in params {
                    resolved_params.push(self.resolve_type_with_bindings(param)?);
                }
                let resolved_return = self.resolve_type_with_bindings(return_type)?;
                Ok(Type::Function(resolved_params, Box::new(resolved_return)))
            }
            Type::Struct(inner_struct) => {
                let mut args = Vec::with_capacity(inner_struct.type_arguments.len());
                for arg in &inner_struct.type_arguments {
                    args.push(self.resolve_type_with_bindings(arg)?);
                }
                Ok(Type::Struct(StructType {
                    name: inner_struct.name.clone(),
                    type_arguments: args,
                }))
            }
            other => Ok(other.clone()),
        }
    }

    fn resolve_type_with_bindings_to_value(&mut self, ty: &Type) -> Result<ValueType> {
        let resolved = self.resolve_type_with_bindings(ty)?;
        if let Type::Struct(struct_type) = &resolved {
            self.ensure_struct_variant_metadata(struct_type)?;
        }
        type_to_value_type(&resolved)
    }

    fn collect_structs(&mut self, statements: &[Statement]) -> Result<()> {
        for statement in statements {
            if let Statement::Struct(struct_stmt) = statement {
                self.structs
                    .entry(struct_stmt.name.clone())
                    .or_insert_with(StructLowering::new);
            }
        }

        for statement in statements {
            if let Statement::Struct(struct_stmt) = statement {
                let field_names: Vec<String> = struct_stmt
                    .fields
                    .iter()
                    .map(|field| field.name.clone())
                    .collect();
                let mut field_types = Vec::new();
                if struct_stmt.type_parameters.is_empty() {
                    field_types = Vec::with_capacity(struct_stmt.fields.len());
                    for field in &struct_stmt.fields {
                        field_types.push(self.parse_type(&field.type_annotation)?);
                    }
                }
                if let Some(entry) = self.structs.get_mut(&struct_stmt.name) {
                    entry.field_names = field_names;
                    entry.field_types = field_types;
                }
                if let Some(entry) = self.structs.get(&struct_stmt.name) {
                    if !entry.field_types.is_empty() {
                        self.struct_field_variants
                            .entry(struct_stmt.name.clone())
                            .or_insert_with(|| entry.field_types.clone());
                    }
                }
                self.struct_variant_bases
                    .entry(struct_stmt.name.clone())
                    .or_insert_with(|| struct_stmt.name.clone());
            }
        }

        let struct_names: Vec<String> = self.structs.keys().cloned().collect();
        for name in struct_names {
            let _ = self.ensure_struct_template(&name)?;
        }
        Ok(())
    }

    fn collect_globals(&mut self, statements: &[Statement]) -> Result<()> {
        for statement in statements {
            if let Statement::Var(var_stmt) = statement {
                for binding in &var_stmt.bindings {
                    let ty = self
                        .binding_types_tc
                        .get(&binding.span)
                        .cloned()
                        .with_context(|| {
                            format!(
                                "missing type information for top-level binding '{}' at {}",
                                binding.name,
                                Self::describe_span(binding.span)
                            )
                        })?;
                    let value_type = self.resolve_type_with_bindings_to_value(&ty)?;
                    let _ =
                        self.ensure_global_slot(&binding.name, &value_type, !var_stmt.is_const)?;
                }
            }
        }
        Ok(())
    }

    fn ensure_struct_template(&mut self, name: &str) -> Result<PointerValue<'ctx>> {
        if let Some(ptr) = self
            .structs
            .get(name)
            .and_then(|entry| entry.template_pointer)
        {
            return Ok(ptr);
        }

        if !self.structs.contains_key(name) {
            if let Some(definition) = self.struct_definitions_tc.get(name) {
                if definition.type_parameters.is_empty() {
                    let mut lowering = StructLowering::new();
                    lowering.field_names = definition
                        .fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect();

                    let mut lowered_types = Vec::with_capacity(definition.fields.len());
                    for field in &definition.fields {
                        lowered_types.push(type_to_value_type(&field.ty)?);
                    }
                    lowering.field_types = lowered_types.clone();

                    self.struct_field_variants
                        .entry(name.to_string())
                        .or_insert_with(|| lowered_types.clone());
                    self.struct_variant_bases
                        .entry(name.to_string())
                        .or_insert_with(|| name.to_string());
                    self.structs.insert(name.to_string(), lowering);
                }
            }
        }

        let field_names = self
            .structs
            .get(name)
            .ok_or_else(|| anyhow!(format!("unknown struct '{name}'")))?
            .field_names
            .clone();

        let name_ptr = self.create_c_string_constant(name);
        let char_ptr_type = self.ptr_type;
        let char_ptr_ptr_type = self.ptr_type;

        let mut field_names_global: Option<GlobalValue<'ctx>> = None;
        let field_names_ptr = if field_names.is_empty() {
            char_ptr_ptr_type.const_null()
        } else {
            let field_ptrs: Vec<_> = field_names
                .iter()
                .map(|field_name| self.create_c_string_constant(field_name))
                .collect();

            let const_array = char_ptr_type.const_array(&field_ptrs);
            let array_type = const_array.get_type();
            let global_name = format!(".struct.fields.{}", name);
            let fields_global = self.module.add_global(array_type, None, &global_name);
            fields_global.set_initializer(&const_array);
            fields_global.set_constant(true);
            fields_global.set_linkage(Linkage::Private);
            field_names_global = Some(fields_global);
            let zero = self.context.i32_type().const_zero();
            unsafe {
                fields_global
                    .as_pointer_value()
                    .const_in_bounds_gep(array_type, &[zero, zero])
            }
        };

        let field_count = self.int_type().const_int(field_names.len() as u64, false);
        let template_value = self.tea_struct_template.const_named_struct(&[
            name_ptr.into(),
            field_count.into(),
            field_names_ptr.into(),
        ]);
        let template_name = format!(".struct.template.{}", name);
        let template_global =
            self.module
                .add_global(self.tea_struct_template, None, &template_name);
        template_global.set_initializer(&template_value);
        template_global.set_constant(true);
        template_global.set_linkage(Linkage::Private);
        let template_ptr = template_global.as_pointer_value();

        if let Some(entry) = self.structs.get_mut(name) {
            entry.template_global = Some(template_global);
            entry.template_pointer = Some(template_ptr);
            entry.field_names_global = field_names_global;
        }

        Ok(template_ptr)
    }

    fn ensure_global_slot(
        &mut self,
        name: &str,
        ty: &ValueType,
        mutable: bool,
    ) -> Result<GlobalValue<'ctx>> {
        if let Some(slot) = self.global_slots.get_mut(name) {
            if slot.ty != *ty {
                bail!(format!(
                    "top-level binding '{}' has conflicting types (previous {:?}, new {:?})",
                    name, slot.ty, ty
                ));
            }
            if slot.mutable != mutable {
                bail!(format!(
                    "top-level binding '{}' has conflicting mutability",
                    name
                ));
            }
            return Ok(slot.pointer);
        }

        let basic = self.basic_type(ty)?;
        let symbol = format!(".binding.{}", sanitize_symbol_component(name));
        let global = self.module.add_global(basic, None, &symbol);
        global.set_linkage(Linkage::Private);
        let initializer = self.zero_value_for_basic(&basic);
        global.set_initializer(&initializer);
        self.global_slots.insert(
            name.to_string(),
            GlobalBindingSlot {
                pointer: global,
                ty: ty.clone(),
                mutable,
                initialized: false,
            },
        );
        Ok(global)
    }

    fn ensure_error_variant_metadata(
        &mut self,
        error_name: &str,
        variant_name: &str,
    ) -> Result<&mut ErrorVariantLowering<'ctx>> {
        let needs_init = self
            .errors
            .get(error_name)
            .and_then(|variants| variants.get(variant_name))
            .map(|entry| entry.field_names.is_empty())
            .unwrap_or(true);

        if needs_init {
            let definition = self
                .error_definitions_tc
                .get(error_name)
                .ok_or_else(|| anyhow!(format!("unknown error '{error_name}'")))?;
            let variant_def = definition.variants.get(variant_name).ok_or_else(|| {
                anyhow!(format!(
                    "error '{error_name}' has no variant '{variant_name}'"
                ))
            })?;

            let field_specs: Vec<(String, Type)> = variant_def
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.ty.clone()))
                .collect();

            let mut field_names = Vec::with_capacity(field_specs.len());
            let mut field_types = Vec::with_capacity(field_specs.len());
            for (name, ty) in field_specs {
                field_names.push(name);
                field_types.push(self.resolve_type_with_bindings_to_value(&ty)?);
            }

            let variants_entry = self
                .errors
                .entry(error_name.to_string())
                .or_insert_with(HashMap::new);
            let entry = variants_entry
                .entry(variant_name.to_string())
                .or_insert_with(ErrorVariantLowering::new);
            entry.field_names = field_names;
            entry.field_types = field_types;
        }

        let variants_entry = self
            .errors
            .entry(error_name.to_string())
            .or_insert_with(HashMap::new);
        let entry = variants_entry
            .entry(variant_name.to_string())
            .or_insert_with(ErrorVariantLowering::new);
        Ok(entry)
    }

    fn ensure_error_template(
        &mut self,
        error_name: &str,
        variant_name: &str,
    ) -> Result<PointerValue<'ctx>> {
        self.ensure_error_variant_metadata(error_name, variant_name)?;

        if let Some(ptr) = self
            .errors
            .get(error_name)
            .and_then(|variants| variants.get(variant_name))
            .and_then(|entry| entry.template_pointer)
        {
            return Ok(ptr);
        }

        let error_name_ptr = self.create_c_string_constant(error_name);
        let variant_name_ptr = self.create_c_string_constant(variant_name);

        let (field_names, existing_names_global) = {
            let entry = self
                .errors
                .get(error_name)
                .and_then(|variants| variants.get(variant_name))
                .ok_or_else(|| {
                    anyhow!(format!(
                        "missing metadata for '{}.{}'",
                        error_name, variant_name
                    ))
                })?;
            (entry.field_names.clone(), entry.field_names_global)
        };

        let field_count_value = self.int_type().const_int(field_names.len() as u64, false);

        let char_ptr_type = self.ptr_type;
        let mut field_names_global = existing_names_global;
        let field_names_ptr = if field_names.is_empty() {
            char_ptr_type.const_null()
        } else {
            let field_ptrs: Vec<_> = field_names
                .iter()
                .map(|field_name| self.create_c_string_constant(field_name))
                .collect();
            let const_array = char_ptr_type.const_array(&field_ptrs);
            let array_type = const_array.get_type();
            let global_name = format!(".error.fields.{}.{}", error_name, variant_name);
            let fields_global = self.module.add_global(array_type, None, &global_name);
            fields_global.set_initializer(&const_array);
            fields_global.set_constant(true);
            fields_global.set_linkage(Linkage::Private);
            field_names_global = Some(fields_global);
            let zero = self.context.i32_type().const_zero();
            unsafe {
                fields_global
                    .as_pointer_value()
                    .const_in_bounds_gep(array_type, &[zero, zero])
            }
        };

        let template_value = self.tea_error_template.const_named_struct(&[
            error_name_ptr.into(),
            variant_name_ptr.into(),
            field_count_value.into(),
            field_names_ptr.into(),
        ]);
        let template_name = format!(".error.template.{}.{}", error_name, variant_name);
        let template_global = self
            .module
            .add_global(self.tea_error_template, None, &template_name);
        template_global.set_initializer(&template_value);
        template_global.set_constant(true);
        template_global.set_linkage(Linkage::Private);
        let template_ptr = template_global.as_pointer_value();

        if let Some(entry) = self
            .errors
            .get_mut(error_name)
            .and_then(|variants| variants.get_mut(variant_name))
        {
            entry.template_global = Some(template_global);
            entry.template_pointer = Some(template_ptr);
            if entry.field_names_global.is_none() {
                entry.field_names_global = field_names_global;
            }
        }

        Ok(template_ptr)
    }

    fn create_c_string_constant(&mut self, value: &str) -> PointerValue<'ctx> {
        let bytes = value.as_bytes();
        let total_len = bytes.len() + 1;
        let array_type = self.context.i8_type().array_type(total_len as u32);
        let mut elems = Vec::with_capacity(total_len);
        for byte in bytes {
            elems.push(self.context.i8_type().const_int(*byte as u64, false));
        }
        elems.push(self.context.i8_type().const_zero());
        let const_array = self.context.i8_type().const_array(&elems);

        let name = format!(".cstr.{}", self.string_counter);
        self.string_counter += 1;
        let global = self.module.add_global(array_type, None, &name);
        global.set_initializer(&const_array);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);

        let zero = self.context.i32_type().const_zero();
        unsafe {
            global
                .as_pointer_value()
                .const_in_bounds_gep(array_type, &[zero, zero])
        }
    }

    fn declare_function(&mut self, function: &FunctionStatement) -> Result<()> {
        if function.type_parameters.is_empty() {
            if self.functions.contains_key(&function.name) {
                return Ok(());
            }

            let return_type = match &function.return_type {
                Some(expr) => self.parse_type(expr)?,
                None => ValueType::Void,
            };
            let can_throw = function.error_annotation.is_some();

            let mut params = Vec::with_capacity(function.parameters.len());
            for param in &function.parameters {
                let type_expr = param.type_annotation.as_ref().ok_or_else(|| {
                    anyhow!("parameter '{}' requires type annotation", param.name)
                })?;
                params.push(self.parse_type(type_expr)?);
            }

            let fn_type = self.function_type(&return_type, &params)?;
            let fn_value = self.module.add_function(&function.name, fn_type, None);

            self.functions.insert(
                function.name.clone(),
                FunctionSignature {
                    value: fn_value,
                    return_type,
                    param_types: params,
                    can_throw,
                },
            );
            return Ok(());
        }

        let instances = match self.function_instances_tc.get(&function.name) {
            Some(instances) if !instances.is_empty() => instances.clone(),
            _ => return Ok(()),
        };

        for instance in instances {
            let mangled = mangle_function_name(&function.name, &instance.type_arguments);
            if self.functions.contains_key(&mangled) {
                continue;
            }

            if let Type::Struct(struct_ty) = &instance.return_type {
                self.ensure_struct_variant_metadata(struct_ty)?;
            }
            let return_type = type_to_value_type(&instance.return_type)?;
            let can_throw = function.error_annotation.is_some();
            let mut params = Vec::with_capacity(instance.param_types.len());
            for param_ty in &instance.param_types {
                if let Type::Struct(struct_ty) = param_ty {
                    self.ensure_struct_variant_metadata(struct_ty)?;
                }
                params.push(type_to_value_type(param_ty)?);
            }

            let fn_type = self.function_type(&return_type, &params)?;
            let fn_value = self.module.add_function(&mangled, fn_type, None);

            self.functions.insert(
                mangled,
                FunctionSignature {
                    value: fn_value,
                    return_type,
                    param_types: params,
                    can_throw,
                },
            );
        }

        Ok(())
    }

    fn compile_function_variants(&mut self, function: &FunctionStatement) -> Result<()> {
        if function.type_parameters.is_empty() {
            return self.compile_function_body(function, &function.name, None);
        }

        let instances = match self.function_instances_tc.get(&function.name) {
            Some(instances) if !instances.is_empty() => instances.clone(),
            _ => return Ok(()),
        };

        for instance in instances.iter() {
            let mangled = mangle_function_name(&function.name, &instance.type_arguments);
            self.compile_function_body(function, &mangled, Some(instance))?;
        }
        Ok(())
    }

    fn compile_function_body(
        &mut self,
        function: &FunctionStatement,
        fn_name: &str,
        instance: Option<&FunctionInstance>,
    ) -> Result<()> {
        let signature = self
            .functions
            .get(fn_name)
            .ok_or_else(|| anyhow!("function '{}' not declared", fn_name))?
            .clone();

        if signature.value.count_basic_blocks() > 0 {
            return Ok(());
        }

        let mut pushed_generics = false;
        if let Some(instance) = instance {
            if function.type_parameters.len() != instance.type_arguments.len() {
                bail!(
                    "function '{}' instantiation parameter mismatch",
                    function.name
                );
            }
            let mut bindings = HashMap::new();
            for (param, ty) in function
                .type_parameters
                .iter()
                .zip(instance.type_arguments.iter())
            {
                let value_type = type_to_value_type(ty)?;
                bindings.insert(param.name.clone(), (ty.clone(), value_type));
            }
            self.generic_binding_stack.push(bindings);
            pushed_generics = true;
        }

        let entry = self.context.append_basic_block(signature.value, "entry");
        self.builder.position_at_end(entry);
        self.push_function_return(signature.return_type.clone());

        let mut locals: HashMap<String, LocalVariable<'ctx>> = HashMap::new();
        for (index, param) in function.parameters.iter().enumerate() {
            let arg = signature.value.get_nth_param(index as u32).expect("param");
            arg.set_name(&param.name);
            let param_type = signature.param_types[index].clone();
            let alloca = self.create_entry_alloca(
                signature.value,
                &param.name,
                self.basic_type(&param_type)?,
            )?;
            map_builder_error(self.builder.build_store(alloca, arg))?;
            locals.insert(
                param.name.clone(),
                LocalVariable {
                    pointer: alloca,
                    ty: param_type,
                    mutable: true,
                },
            );
        }

        let result = (|| -> Result<()> {
            let terminated = self.compile_block(
                &function.body.statements,
                signature.value,
                &mut locals,
                &signature.return_type,
                false,
            )?;

            if !terminated {
                match &signature.return_type {
                    ValueType::Void => {
                        self.clear_error_state()?;
                        map_builder_error(self.builder.build_return(None))?;
                    }
                    ValueType::Int => {
                        bail!(
                            "function '{}' may exit without returning Int",
                            function.name
                        )
                    }
                    ValueType::Float => {
                        bail!(
                            "function '{}' may exit without returning Float",
                            function.name
                        )
                    }
                    ValueType::Bool => {
                        bail!(
                            "function '{}' may exit without returning Bool",
                            function.name
                        )
                    }
                    ValueType::String => {
                        bail!(
                            "function '{}' may exit without returning String",
                            function.name
                        )
                    }
                    ValueType::List(_) => {
                        bail!(
                            "function '{}' may exit without returning List",
                            function.name
                        )
                    }
                    ValueType::Dict(_) => {
                        bail!(
                            "function '{}' may exit without returning Dict",
                            function.name
                        )
                    }
                    ValueType::Struct(_) => {
                        bail!(
                            "function '{}' may exit without returning Struct",
                            function.name
                        )
                    }
                    ValueType::Error { .. } => {
                        bail!(
                            "function '{}' may exit without returning Error value",
                            function.name
                        )
                    }
                    ValueType::Function(_, _) => {
                        bail!(
                            "function '{}' may exit without returning function value",
                            function.name
                        )
                    }
                    ValueType::Optional(_) => {
                        bail!(
                            "function '{}' may exit without returning Optional value",
                            function.name
                        )
                    }
                }
            }

            Ok(())
        })();

        if pushed_generics {
            self.generic_binding_stack.pop();
        }
        self.pop_function_return();

        result
    }

    fn compile_main(&mut self, statements: &[Statement]) -> Result<()> {
        let fn_type = self.context.i32_type().fn_type(&[], false);
        let main_fn = self.module.add_function("main", fn_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);

        let mut locals: HashMap<String, LocalVariable<'ctx>> = HashMap::new();
        let return_type = ValueType::Int;
        self.push_function_return(return_type.clone());
        for statement in statements {
            match statement {
                Statement::Use(_)
                | Statement::Function(_)
                | Statement::Test(_)
                | Statement::Enum(_) => {}
                Statement::Var(var_stmt) => {
                    self.compile_global_var(var_stmt, main_fn, &mut locals)?;
                }
                Statement::Return(_) => bail!("return at top level not supported"),
                _ => {
                    let _ = self.compile_statement(
                        statement,
                        main_fn,
                        &mut locals,
                        &return_type,
                        false,
                    )?;
                }
            }
        }

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.clear_error_state()?;
            map_builder_error(
                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(0, false))),
            )?;
        }

        self.pop_function_return();

        Ok(())
    }

    fn compile_block(
        &mut self,
        statements: &[Statement],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
        new_scope: bool,
    ) -> Result<bool> {
        let mut scope_locals;
        let locals = if new_scope {
            scope_locals = locals.clone();
            &mut scope_locals
        } else {
            locals
        };

        for (index, statement) in statements.iter().enumerate() {
            let is_last = index + 1 == statements.len();
            if self.compile_statement(statement, function, locals, return_type, is_last)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn compile_statement(
        &mut self,
        statement: &Statement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
        is_last: bool,
    ) -> Result<bool> {
        match statement {
            Statement::Expression(expr) => {
                let value = self.compile_expression(&expr.expression, function, locals)?;
                if is_last && !matches!(return_type, ValueType::Void) && value.ty() == *return_type
                {
                    if let Some(basic) = value.into_basic_value() {
                        self.clear_error_state()?;
                        map_builder_error(self.builder.build_return(Some(&basic)))?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Statement::Var(var_stmt) => {
                self.compile_var(var_stmt, function, locals)?;
                Ok(false)
            }
            Statement::Return(ret) => {
                self.compile_return(ret, function, locals, return_type)?;
                Ok(true)
            }
            Statement::Conditional(cond) => {
                self.compile_conditional(cond, function, locals, return_type)
            }
            Statement::Loop(loop_stmt) => {
                self.compile_loop(loop_stmt, function, locals, return_type)
            }
            Statement::Struct(_) => Ok(false),
            Statement::Union(_) => Ok(false),
            Statement::Enum(_) => Ok(false),
            Statement::Test(_) => Ok(false),
            Statement::Match(_) => {
                bail!("match statements are not supported by the AOT backend yet")
            }
            Statement::Use(_) | Statement::Function(_) => {
                bail!("unsupported statement in function body")
            }
            Statement::Error(_) => Ok(false),
            Statement::Throw(throw_stmt) => {
                self.compile_throw(throw_stmt, function, locals, return_type)?;
                Ok(true)
            }
        }
    }

    fn compile_global_var(
        &mut self,
        statement: &VarStatement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<()> {
        for binding in &statement.bindings {
            let initializer = match binding.initializer.as_ref() {
                Some(expr) => expr,
                None if statement.is_const => {
                    bail!(format!(
                        "const '{}' requires an initializer (at {})",
                        binding.name,
                        Self::describe_span(binding.span)
                    ));
                }
                None => {
                    bail!(format!(
                        "variable '{}' requires an initializer (at {})",
                        binding.name,
                        Self::describe_span(binding.span)
                    ));
                }
            };

            let value = self.compile_expression(initializer, function, locals)?;
            let (ty, initial_value) = match binding.type_annotation.as_ref() {
                Some(type_expr) => {
                    let expected = self.parse_type(type_expr)?;
                    let init_type = value.ty();
                    let converted = self
                        .convert_expr_to_type(value, &expected)
                        .with_context(|| {
                            format!(
                                "initializer for '{}' has mismatched type (expected {:?}, found {:?})",
                                binding.name, expected, init_type
                            )
                        })?;
                    (expected, converted)
                }
                None => {
                    let ty = value.ty();
                    (ty, value)
                }
            };

            let global = self.ensure_global_slot(&binding.name, &ty, !statement.is_const)?;
            self.store_expr_in_pointer(
                global.as_pointer_value(),
                &ty,
                initial_value,
                &binding.name,
            )?;
            if let Some(slot) = self.global_slots.get_mut(&binding.name) {
                slot.initialized = true;
                slot.mutable = !statement.is_const;
                slot.ty = ty.clone();
            }
            locals.insert(
                binding.name.clone(),
                LocalVariable {
                    pointer: global.as_pointer_value(),
                    ty,
                    mutable: !statement.is_const,
                },
            );
        }
        Ok(())
    }

    fn compile_var(
        &mut self,
        statement: &VarStatement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<()> {
        for binding in &statement.bindings {
            if locals.contains_key(&binding.name) {
                bail!("variable '{}' already exists", binding.name);
            }
            let initializer = match binding.initializer.as_ref() {
                Some(expr) => expr,
                None if statement.is_const => {
                    bail!("const '{}' requires an initializer", binding.name);
                }
                None => {
                    bail!("variable '{}' requires an initializer", binding.name);
                }
            };
            let value = self.compile_expression(initializer, function, locals)?;
            let (ty, initial_value) = match binding.type_annotation.as_ref() {
                Some(type_expr) => {
                    let expected = self.parse_type(type_expr)?;
                    let init_type = value.ty();
                    let converted = self
                        .convert_expr_to_type(value, &expected)
                        .with_context(|| {
                            format!(
                                "initializer for '{}' has mismatched type (expected {:?}, found {:?})",
                                binding.name, expected, init_type
                            )
                        })?;
                    (expected, converted)
                }
                None => {
                    let ty = value.ty();
                    (ty, value)
                }
            };

            let alloca =
                self.create_entry_alloca(function, &binding.name, self.basic_type(&ty)?)?;
            self.store_expr_in_pointer(alloca, &ty, initial_value, &binding.name)?;
            locals.insert(
                binding.name.clone(),
                LocalVariable {
                    pointer: alloca,
                    ty,
                    mutable: !statement.is_const,
                },
            );
        }
        Ok(())
    }

    fn compile_return(
        &mut self,
        statement: &ReturnStatement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
    ) -> Result<()> {
        match (&statement.expression, return_type) {
            (Some(_), ValueType::Void) => bail!("return with value in void function"),
            (None, ValueType::Void) => {
                self.clear_error_state()?;
                map_builder_error(self.builder.build_return(None))?;
                return Ok(());
            }
            (Some(expr), ty) => {
                let value = self.compile_expression(expr, function, locals)?;
                let converted = self.convert_expr_to_type(value, ty)?;
                self.emit_return_value(converted, ty)
            }
            (None, ValueType::Int) => {
                if let Some(ret_ty) = function.get_type().get_return_type() {
                    if let BasicTypeEnum::IntType(int_ty) = ret_ty {
                        let zero = int_ty.const_zero();
                        self.clear_error_state()?;
                        map_builder_error(self.builder.build_return(Some(&zero)))?;
                    } else {
                        let zero = self.int_type().const_int(0, false);
                        self.clear_error_state()?;
                        map_builder_error(self.builder.build_return(Some(&zero)))?;
                    }
                } else {
                    let zero = self.int_type().const_int(0, false);
                    self.clear_error_state()?;
                    map_builder_error(self.builder.build_return(Some(&zero)))?;
                }
                Ok(())
            }
            (None, ValueType::Optional(inner)) => {
                let nil_value = self.optional_nil(inner)?;
                self.emit_return_value(nil_value, return_type)
            }
            (None, ValueType::Function(_, _)) => bail!("missing return value"),
            _ => bail!("missing return value"),
        }
    }

    fn compile_throw(
        &mut self,
        statement: &ThrowStatement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
    ) -> Result<()> {
        let value = self.compile_expression(&statement.expression, function, locals)?;
        let ExprValue::Error { pointer, .. } = value else {
            bail!("throw expression must evaluate to an error value");
        };
        let set_fn = self.ensure_error_set_current();
        self.call_function(set_fn, &[pointer.into()], "error_set_current")?;
        self.emit_error_return(function, return_type)
    }

    fn compile_conditional(
        &mut self,
        statement: &ConditionalStatement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
    ) -> Result<bool> {
        let mut condition = self
            .compile_expression(&statement.condition, function, locals)?
            .into_bool()?;
        if matches!(statement.kind, ConditionalKind::Unless) {
            condition = map_builder_error(self.builder.build_not(condition, "unless"))?;
        }

        let then_block = self.context.append_basic_block(function, "if_then");
        let else_block = statement
            .alternative
            .as_ref()
            .map(|_| self.context.append_basic_block(function, "if_else"));
        let merge_block = self.context.append_basic_block(function, "if_merge");

        map_builder_error(self.builder.build_conditional_branch(
            condition,
            then_block,
            else_block.unwrap_or(merge_block),
        ))?;

        self.builder.position_at_end(then_block);
        let then_terminated = self.compile_block(
            &statement.consequent.statements,
            function,
            locals,
            return_type,
            true,
        )?;
        if !then_terminated {
            map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
        }
        let _then_end = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing then block"))?;

        let else_terminated = if let Some(else_block) = else_block {
            self.builder.position_at_end(else_block);
            let terminated = self.compile_block(
                &statement.alternative.as_ref().expect("alt").statements,
                function,
                locals,
                return_type,
                true,
            )?;
            if !terminated {
                map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
            }
            terminated
        } else {
            false
        };
        let _else_end = if let Some(_block) = else_block {
            Some(
                self.builder
                    .get_insert_block()
                    .ok_or_else(|| anyhow!("missing else block"))?,
            )
        } else {
            None
        };

        if then_terminated && else_terminated {
            Ok(true)
        } else {
            self.builder.position_at_end(merge_block);
            Ok(false)
        }
    }

    fn compile_loop(
        &mut self,
        statement: &LoopStatement,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
    ) -> Result<bool> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;

        let cond_block = self.context.append_basic_block(function, "loop_cond");
        let body_block = self.context.append_basic_block(function, "loop_body");
        let exit_block = self.context.append_basic_block(function, "loop_exit");

        if current_block.get_terminator().is_none() {
            map_builder_error(self.builder.build_unconditional_branch(cond_block))?;
        }

        self.builder.position_at_end(cond_block);
        let cond_expr = match &statement.header {
            LoopHeader::Condition(expr) => expr,
            LoopHeader::For { .. } => bail!("for loops unsupported"),
        };
        let mut cond_value = self
            .compile_expression(cond_expr, function, locals)?
            .into_bool()?;
        if matches!(statement.kind, LoopKind::Until) {
            cond_value = map_builder_error(self.builder.build_not(cond_value, "until"))?;
        }

        map_builder_error(
            self.builder
                .build_conditional_branch(cond_value, body_block, exit_block),
        )?;

        self.builder.position_at_end(body_block);
        let body_terminated = self.compile_block(
            &statement.body.statements,
            function,
            locals,
            return_type,
            true,
        )?;
        if !body_terminated {
            map_builder_error(self.builder.build_unconditional_branch(cond_block))?;
        }

        self.builder.position_at_end(exit_block);
        Ok(false)
    }

    fn compile_assignment(
        &mut self,
        assignment: &crate::ast::AssignmentExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match &assignment.target.kind {
            ExpressionKind::Identifier(identifier) => {
                if let Some(variable) = locals.get(identifier.name.as_str()) {
                    if !variable.mutable {
                        bail!(format!(
                            "cannot assign to const '{}' at {}",
                            identifier.name,
                            Self::describe_span(assignment.target.span)
                        ));
                    }
                    let pointer = variable.pointer;
                    let var_ty = variable.ty.clone();
                    let value = self.compile_expression(&assignment.value, function, locals)?;
                    self.store_expr_in_pointer(pointer, &var_ty, value, &identifier.name)?;
                    Ok(ExprValue::Void)
                } else if let Some(slot) = self.global_slots.get(identifier.name.as_str()).cloned()
                {
                    if !slot.mutable {
                        bail!(format!(
                            "cannot assign to const '{}' at {}",
                            identifier.name,
                            Self::describe_span(assignment.target.span)
                        ));
                    }
                    let pointer = slot.pointer.as_pointer_value();
                    let ty = slot.ty.clone();
                    let value = self.compile_expression(&assignment.value, function, locals)?;
                    self.store_expr_in_pointer(pointer, &ty, value, &identifier.name)?;
                    if let Some(slot_mut) = self.global_slots.get_mut(identifier.name.as_str()) {
                        slot_mut.initialized = true;
                    }
                    Ok(ExprValue::Void)
                } else {
                    Err(self.undefined_identifier_error(&identifier.name, assignment.target.span))
                }
            }
            _ => bail!("only identifier assignment supported"),
        }
    }

    fn compile_list_literal(
        &mut self,
        list: &crate::ast::ListLiteral,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let alloc_fn = self.ensure_alloc_list();
        let call = self.call_function(
            alloc_fn,
            &[self
                .int_type()
                .const_int(list.elements.len() as u64, false)
                .into()],
            "list_alloc",
        )?;
        let list_ptr = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected list pointer"))?
            .into_pointer_value();

        let list_set = self.ensure_list_set();
        let mut element_type: Option<ValueType> = None;
        for (index, element) in list.elements.iter().enumerate() {
            let expr = self.compile_expression(element, function, locals)?;
            let expr_type = expr.ty();
            if let Some(existing) = &element_type {
                if *existing != expr_type {
                    bail!("list literal elements must share a type");
                }
            } else {
                element_type = Some(expr_type.clone());
            }
            let tea_value = BasicMetadataValueEnum::from(self.expr_to_tea_value(expr)?);
            let index_value = self.int_type().const_int(index as u64, false);
            self.call_function(
                list_set,
                &[list_ptr.into(), index_value.into(), tea_value],
                "list_set",
            )?;
        }

        let element_type = element_type.unwrap_or(ValueType::Void);
        Ok(ExprValue::List {
            pointer: list_ptr,
            element_type: Box::new(element_type),
        })
    }

    fn expect_string_pointer(
        &self,
        expr: ExprValue<'ctx>,
        context: &str,
    ) -> Result<PointerValue<'ctx>> {
        match expr {
            ExprValue::String(ptr) => Ok(ptr),
            _ => bail!(context.to_string()),
        }
    }

    fn expect_int_value(&self, expr: ExprValue<'ctx>, context: &str) -> Result<IntValue<'ctx>> {
        match expr {
            ExprValue::Int(value) => Ok(value),
            _ => bail!(context.to_string()),
        }
    }

    fn compile_dict_literal(
        &mut self,
        dict: &crate::ast::DictLiteral,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let dict_new = self.ensure_dict_new();
        let dict_ptr = self
            .call_function(dict_new, &[], "dict_new")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected dict pointer"))?
            .into_pointer_value();

        let dict_set = self.ensure_dict_set();

        let mut value_type: Option<ValueType> = None;
        for entry in &dict.entries {
            let key_expr = self.compile_string_literal(&entry.key)?;
            let key_ptr = match key_expr {
                ExprValue::String(ptr) => ptr,
                _ => bail!("dict key literal did not lower to string"),
            };

            let value_expr = self.compile_expression(&entry.value, function, locals)?;
            let value_ty = value_expr.ty();
            if let Some(existing) = &value_type {
                if *existing != value_ty {
                    bail!("dict literal values must share a type");
                }
            } else {
                value_type = Some(value_ty.clone());
            }

            let tea_value = self.expr_to_tea_value(value_expr)?;
            self.call_function(
                dict_set,
                &[
                    dict_ptr.into(),
                    key_ptr.into(),
                    BasicMetadataValueEnum::from(tea_value),
                ],
                "dict_set",
            )?;
        }

        let value_type = value_type.unwrap_or(ValueType::Void);
        Ok(ExprValue::Dict {
            pointer: dict_ptr,
            value_type: Box::new(value_type),
        })
    }

    fn compile_index(
        &mut self,
        index: &crate::ast::IndexExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let object = self.compile_expression(&index.object, function, locals)?;
        let key_expr = self.compile_expression(&index.index, function, locals)?;
        match object {
            ExprValue::List {
                pointer,
                element_type,
            } => {
                let index_value = key_expr.into_int()?;
                let list_get = self.ensure_list_get();
                let tea_value = self
                    .call_function(list_get, &[pointer.into(), index_value.into()], "list_get")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?
                    .into_struct_value();
                self.tea_value_to_expr(tea_value, *element_type)
            }
            ExprValue::Dict {
                pointer,
                value_type,
            } => {
                let key_ptr = match key_expr {
                    ExprValue::String(ptr) => ptr,
                    _ => bail!("dictionary index expects a String key"),
                };
                let dict_get = self.ensure_dict_get();
                let tea_value = self
                    .call_function(dict_get, &[pointer.into(), key_ptr.into()], "dict_get")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue from dict_get"))?
                    .into_struct_value();
                self.tea_value_to_expr(tea_value, *value_type)
            }
            _ => bail!("indexing expects a list value"),
        }
    }

    fn compile_member(
        &mut self,
        member: &crate::ast::MemberExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let object = self.compile_expression(&member.object, function, locals)?;
        match object {
            ExprValue::Struct {
                pointer,
                struct_name,
            } => {
                let base_name = self
                    .struct_variant_bases
                    .get(&struct_name)
                    .cloned()
                    .unwrap_or_else(|| struct_name.clone());
                let info = self
                    .structs
                    .get(&base_name)
                    .ok_or_else(|| anyhow!(format!("unknown struct '{base_name}'")))?;
                let field_types_vec = self
                    .struct_field_variants
                    .get(&struct_name)
                    .or_else(|| self.struct_field_variants.get(&base_name))
                    .cloned()
                    .unwrap_or_else(|| info.field_types.clone());
                let index = info.field_index(&member.property).ok_or_else(|| {
                    anyhow!(format!(
                        "struct '{}' has no field '{}'",
                        base_name, member.property
                    ))
                })?;
                let field_type = field_types_vec
                    .get(index)
                    .cloned()
                    .ok_or_else(|| anyhow!("missing field type metadata"))?;
                let get_fn = self.ensure_struct_get();
                let tea_value = self
                    .call_function(
                        get_fn,
                        &[
                            pointer.into(),
                            self.int_type().const_int(index as u64, false).into(),
                        ],
                        "struct_get",
                    )?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue from struct_get"))?
                    .into_struct_value();
                self.tea_value_to_expr(tea_value, field_type)
            }
            ExprValue::Dict {
                pointer,
                value_type,
            } => {
                let key_expr = self.compile_string_literal(&member.property)?;
                let key_ptr = match key_expr {
                    ExprValue::String(ptr) => ptr,
                    _ => bail!("dict member lookup failed to lower property name"),
                };
                let dict_get = self.ensure_dict_get();
                let tea_value = self
                    .call_function(
                        dict_get,
                        &[pointer.into(), key_ptr.into()],
                        "dict_get_member",
                    )?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue from dict_get"))?
                    .into_struct_value();
                self.tea_value_to_expr(tea_value, *value_type)
            }
            ExprValue::Error {
                pointer,
                error_name,
                variant_name,
            } => {
                let variant = variant_name.clone().ok_or_else(|| {
                    anyhow!(
                        "cannot access field '{}' on error '{}' without a specific variant",
                        member.property,
                        error_name
                    )
                })?;
                let variant_entry = self.ensure_error_variant_metadata(&error_name, &variant)?;
                let index = variant_entry
                    .field_names
                    .iter()
                    .position(|field| field == &member.property)
                    .ok_or_else(|| {
                        anyhow!(
                            "error '{}.{}' has no field '{}'",
                            error_name,
                            variant,
                            member.property
                        )
                    })?;
                let field_type = variant_entry
                    .field_types
                    .get(index)
                    .cloned()
                    .ok_or_else(|| anyhow!("missing field type metadata for error field"))?;
                let get_fn = self.ensure_error_get();
                let tea_value = self
                    .call_function(
                        get_fn,
                        &[
                            pointer.into(),
                            self.int_type().const_int(index as u64, false).into(),
                        ],
                        "error_get_field",
                    )?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue from error_get_field"))?
                    .into_struct_value();
                self.tea_value_to_expr(tea_value, field_type)
            }
            _ => bail!("member access expects a struct value"),
        }
    }

    fn compile_lambda_expression(
        &mut self,
        lambda: &LambdaExpression,
        _function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let signature = self
            .lambda_signatures
            .get(&lambda.id)
            .cloned()
            .ok_or_else(|| anyhow!(format!("missing lambda signature for id {}", lambda.id)))?;

        let capture_names = self
            .lambda_captures
            .get(&lambda.id)
            .cloned()
            .unwrap_or_else(Vec::new);

        let capture_types = if let Some(existing) = self.lambda_capture_types.get(&lambda.id) {
            existing.clone()
        } else {
            let mut types = Vec::with_capacity(capture_names.len());
            for name in &capture_names {
                let variable = locals.get(name.as_str()).ok_or_else(|| {
                    anyhow!(format!(
                        "capture '{}' is undefined in lambda {}",
                        name, lambda.id
                    ))
                })?;
                types.push(variable.ty.clone());
            }
            self.lambda_capture_types.insert(lambda.id, types.clone());
            types
        };

        if capture_types.len() != capture_names.len() {
            bail!("capture metadata mismatch for lambda {}", lambda.id);
        }

        let saved_block = self.builder.get_insert_block();
        let lambda_fn =
            self.ensure_lambda_function(lambda, &capture_names, &capture_types, &signature)?;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }

        let fn_ptr = lambda_fn.as_global_value().as_pointer_value();
        let raw_ptr = map_builder_error(self.builder.build_bit_cast(
            fn_ptr,
            self.ptr_type,
            "lambda_fn_ptr",
        ))?
        .into_pointer_value();

        let closure_new = self.ensure_closure_new();
        let capture_count = self.int_type().const_int(capture_names.len() as u64, false);
        let closure_ptr = self
            .call_function(
                closure_new,
                &[raw_ptr.into(), capture_count.into()],
                "closure_new",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected closure pointer"))?
            .into_pointer_value();

        let set_fn = self.ensure_closure_set();
        for (index, (name, _capture_type)) in
            capture_names.iter().zip(capture_types.iter()).enumerate()
        {
            let variable = locals.get(name.as_str()).ok_or_else(|| {
                anyhow!(format!(
                    "capture '{}' is undefined in lambda {}",
                    name, lambda.id
                ))
            })?;
            let value = self.load_local_variable(name, variable)?;
            let tea_value = self.expr_to_tea_value(value)?;
            let index_const = self.int_type().const_int(index as u64, false);
            self.call_function(
                set_fn,
                &[
                    closure_ptr.into(),
                    index_const.into(),
                    BasicMetadataValueEnum::from(tea_value),
                ],
                &format!("closure_set_{index}"),
            )?;
        }

        Ok(ExprValue::Closure {
            pointer: closure_ptr,
            param_types: signature.param_types.clone(),
            return_type: Box::new(signature.return_type.clone()),
        })
    }

    fn compile_try_expression(
        &mut self,
        expression: &TryExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let Some(clause) = &expression.catch else {
            return self.compile_expression(&expression.expression, function, locals);
        };

        self.push_error_mode(ErrorHandlingMode::Capture);
        let success_value = self.compile_expression(&expression.expression, function, locals)?;
        self.pop_error_mode();

        let result_type = success_value.ty();
        let needs_storage = !matches!(result_type, ValueType::Void);
        let result_alloca = if needs_storage {
            Some(self.create_entry_alloca(
                function,
                "try_result",
                self.basic_type(&result_type)?,
            )?)
        } else {
            None
        };

        let error_fn = self.ensure_error_current();
        let call = self.call_function(error_fn, &[], "error_current")?;
        let error_ptr = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_error_current returned no value"))?
            .into_pointer_value();

        let is_null =
            map_builder_error(self.builder.build_is_null(error_ptr, "try_error_is_null"))?;
        let success_block = self.context.append_basic_block(function, "try_success");
        let catch_block = self.context.append_basic_block(function, "try_catch");
        let merge_block = self.context.append_basic_block(function, "try_merge");

        map_builder_error(self.builder.build_conditional_branch(
            is_null,
            success_block,
            catch_block,
        ))?;

        self.builder.position_at_end(success_block);
        if let Some(alloca) = result_alloca {
            self.store_expr_in_pointer(alloca, &result_type, success_value, "try_result")?;
        } else if !matches!(result_type, ValueType::Void) {
            bail!("try expression expected to produce a value");
        }
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
        self.builder.position_at_end(catch_block);
        match &clause.kind {
            CatchKind::Fallback(fallback) => {
                let clear_fn = self.ensure_error_clear_current();
                self.call_function(clear_fn, &[], "error_clear_current")?;
                let fallback_value = self.compile_expression(fallback, function, locals)?;
                if let Some(alloca) = result_alloca {
                    self.store_expr_in_pointer(alloca, &result_type, fallback_value, "try_result")?;
                } else if !matches!(fallback_value.ty(), ValueType::Void) {
                    bail!("catch fallback must evaluate to a Void value");
                }
                map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
            }
            CatchKind::Arms(arms) => {
                let template_fn = self.ensure_error_get_template();
                let template_ptr = self
                    .call_function(template_fn, &[error_ptr.into()], "error_template")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("tea_error_get_template returned no value"))?
                    .into_pointer_value();

                let mut catch_locals_base = locals.clone();
                let binding_info = if let Some(binding) = &clause.binding {
                    let binding_type = self
                        .binding_types_tc
                        .get(&binding.span)
                        .ok_or_else(|| {
                            anyhow!(format!(
                                "missing type metadata for catch binding '{}'",
                                binding.name
                            ))
                        })?
                        .clone();
                    let binding_value_type =
                        self.resolve_type_with_bindings_to_value(&binding_type)?;
                    if !matches!(binding_value_type, ValueType::Error { .. }) {
                        bail!(format!(
                            "catch binding '{}' must resolve to an error type",
                            binding.name
                        ));
                    }
                    let alloca = self.create_entry_alloca(
                        function,
                        &binding.name,
                        self.basic_type(&binding_value_type)?,
                    )?;
                    catch_locals_base.insert(
                        binding.name.clone(),
                        LocalVariable {
                            pointer: alloca,
                            ty: binding_value_type.clone(),
                            mutable: true,
                        },
                    );
                    Some((binding.name.clone(), alloca, binding_value_type))
                } else {
                    None
                };

                let clear_fn = self.ensure_error_clear_current();

                for (index, arm) in arms.iter().enumerate() {
                    let cond = self.build_catch_pattern_condition(template_ptr, &arm.patterns)?;
                    let arm_block = self
                        .context
                        .append_basic_block(function, &format!("catch_arm_{index}"));
                    let next_block = self
                        .context
                        .append_basic_block(function, &format!("catch_next_{index}"));
                    map_builder_error(
                        self.builder
                            .build_conditional_branch(cond, arm_block, next_block),
                    )?;

                    self.builder.position_at_end(arm_block);
                    let mut arm_locals = catch_locals_base.clone();
                    if let Some((ref binding_name, binding_alloca, ref binding_base_type)) =
                        binding_info
                    {
                        let arm_binding_type =
                            self.determine_binding_type_for_arm(binding_base_type, &arm.patterns)?;
                        arm_locals.insert(
                            binding_name.clone(),
                            LocalVariable {
                                pointer: binding_alloca,
                                ty: arm_binding_type.clone(),
                                mutable: true,
                            },
                        );
                        if let ValueType::Error {
                            error_name,
                            variant_name,
                        } = &arm_binding_type
                        {
                            let binding_value = ExprValue::Error {
                                pointer: error_ptr,
                                error_name: error_name.clone(),
                                variant_name: variant_name.clone(),
                            };
                            self.store_expr_in_pointer(
                                binding_alloca,
                                &arm_binding_type,
                                binding_value,
                                binding_name,
                            )?;
                        }
                    }

                    match &arm.handler {
                        CatchHandler::Expression(expr) => {
                            self.call_function(clear_fn, &[], "error_clear_current")?;
                            let arm_value =
                                self.compile_expression(expr, function, &mut arm_locals)?;
                            if let Some(alloca) = result_alloca {
                                self.store_expr_in_pointer(
                                    alloca,
                                    &result_type,
                                    arm_value,
                                    "try_result",
                                )?;
                            } else if !matches!(arm_value.ty(), ValueType::Void) {
                                bail!("catch arm expression must evaluate to a Void value");
                            }
                            map_builder_error(
                                self.builder.build_unconditional_branch(merge_block),
                            )?;
                        }
                        CatchHandler::Block(block) => {
                            let function_return_type = self.current_function_return_type().clone();
                            let _ = self.compile_block(
                                &block.statements,
                                function,
                                &mut arm_locals,
                                &function_return_type,
                                true,
                            )?;
                            let block_terminated = self
                                .builder
                                .get_insert_block()
                                .and_then(|b| b.get_terminator())
                                .is_some();
                            if !block_terminated {
                                self.emit_error_return(function, &function_return_type)?;
                            }
                        }
                    }

                    self.builder.position_at_end(next_block);
                }

                let function_return_type = self.current_function_return_type().clone();
                let block_terminated = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_terminator())
                    .is_some();
                if !block_terminated {
                    self.emit_error_return(function, &function_return_type)?;
                }
            }
        }
        self.builder.position_at_end(merge_block);

        if let Some(alloca) = result_alloca {
            let result_var = LocalVariable {
                pointer: alloca,
                ty: result_type.clone(),
                mutable: true,
            };
            self.load_local_variable("try_result", &result_var)
        } else {
            Ok(ExprValue::Void)
        }
    }

    fn build_catch_pattern_condition(
        &mut self,
        template_ptr: PointerValue<'ctx>,
        patterns: &[MatchPattern],
    ) -> Result<IntValue<'ctx>> {
        if patterns.is_empty() {
            return Ok(self.bool_type().const_zero());
        }

        let mut condition: Option<IntValue<'ctx>> = None;
        for (index, pattern) in patterns.iter().enumerate() {
            let cond = match pattern {
                MatchPattern::Wildcard { .. } => self.bool_type().const_int(1, false),
                MatchPattern::Type(_, span) => {
                    let ty = self
                        .type_test_metadata_tc
                        .get(span)
                        .ok_or_else(|| anyhow!("missing type metadata for catch pattern"))?
                        .clone();
                    let value_type = self.resolve_type_with_bindings_to_value(&ty)?;
                    self.build_error_type_condition(template_ptr, &value_type)?
                }
                _ => {
                    bail!("catch patterns may only match error variants or use '_'");
                }
            };
            condition = Some(match condition {
                Some(existing) => map_builder_error(self.builder.build_or(
                    existing,
                    cond,
                    &format!("catch_pattern_or_{index}"),
                ))?,
                None => cond,
            });
        }

        condition.ok_or_else(|| anyhow!("failed to build catch pattern condition"))
    }

    fn build_error_type_condition(
        &mut self,
        template_ptr: PointerValue<'ctx>,
        pattern_type: &ValueType,
    ) -> Result<IntValue<'ctx>> {
        let ValueType::Error {
            error_name,
            variant_name,
        } = pattern_type
        else {
            bail!("catch pattern must resolve to an error type");
        };

        if let Some(variant) = variant_name {
            let expected = self.ensure_error_template(error_name, variant)?;
            self.pointer_equals(template_ptr, expected, "catch_variant_match")
        } else {
            let definition = self
                .error_definitions_tc
                .get(error_name)
                .ok_or_else(|| anyhow!(format!("unknown error '{}'", error_name)))?;
            let variant_names: Vec<String> = definition.variants.keys().cloned().collect();
            let mut combined: Option<IntValue<'ctx>> = None;
            for variant in variant_names {
                let expected = self.ensure_error_template(error_name, &variant)?;
                let cmp = self.pointer_equals(template_ptr, expected, "catch_variant_match")?;
                combined = Some(match combined {
                    Some(existing) => map_builder_error(self.builder.build_or(
                        existing,
                        cmp,
                        "catch_variant_any",
                    ))?,
                    None => cmp,
                });
            }
            combined.ok_or_else(|| anyhow!(format!("error '{}' has no variants", error_name)))
        }
    }

    fn determine_binding_type_for_arm(
        &mut self,
        binding_base: &ValueType,
        patterns: &[MatchPattern],
    ) -> Result<ValueType> {
        let ValueType::Error {
            error_name: base_name,
            variant_name: base_variant,
        } = binding_base
        else {
            return Ok(binding_base.clone());
        };

        let mut current_variant = base_variant.clone();
        let mut saw_type_pattern = false;

        for pattern in patterns {
            match pattern {
                MatchPattern::Type(_, span) => {
                    let ty = self
                        .type_test_metadata_tc
                        .get(span)
                        .ok_or_else(|| anyhow!("missing type metadata for catch pattern"))?
                        .clone();
                    let value_type = self.resolve_type_with_bindings_to_value(&ty)?;
                    let ValueType::Error {
                        error_name: pattern_name,
                        variant_name: pattern_variant,
                    } = value_type
                    else {
                        bail!("catch pattern must resolve to an error variant");
                    };
                    if pattern_name != *base_name {
                        return Ok(binding_base.clone());
                    }
                    saw_type_pattern = true;
                    current_variant = match (&current_variant, pattern_variant.clone()) {
                        (Some(existing), Some(pattern_variant)) if existing == &pattern_variant => {
                            Some(existing.clone())
                        }
                        (Some(_), Some(_)) => None,
                        (None, Some(pattern_variant)) => Some(pattern_variant),
                        (_, None) => None,
                    };
                }
                MatchPattern::Wildcard { .. } => {
                    return Ok(binding_base.clone());
                }
                _ => {
                    return Ok(binding_base.clone());
                }
            }
        }

        if !saw_type_pattern {
            return Ok(binding_base.clone());
        }

        Ok(ValueType::Error {
            error_name: base_name.clone(),
            variant_name: current_variant,
        })
    }

    fn ensure_lambda_function(
        &mut self,
        lambda: &LambdaExpression,
        capture_names: &[String],
        capture_types: &[ValueType],
        signature: &LambdaSignature,
    ) -> Result<FunctionValue<'ctx>> {
        if let Some(existing) = self.lambda_functions.get(&lambda.id).cloned() {
            return Ok(existing);
        }

        let fn_name = format!("lambda_{}", lambda.id);
        let mut param_types = Vec::with_capacity(signature.param_types.len() + 1);
        param_types.push(self.closure_ptr_type().into());
        for param in &signature.param_types {
            param_types.push(self.basic_type(param)?.into());
        }

        let fn_type = match &signature.return_type {
            ValueType::Void => self.context.void_type().fn_type(&param_types, false),
            ValueType::Int => self.int_type().fn_type(&param_types, false),
            ValueType::Float => self.float_type().fn_type(&param_types, false),
            ValueType::Bool => self.bool_type().fn_type(&param_types, false),
            ValueType::String => self.string_ptr_type().fn_type(&param_types, false),
            ValueType::List(_) => self.list_ptr_type().fn_type(&param_types, false),
            ValueType::Dict(_) => self.dict_ptr_type().fn_type(&param_types, false),
            ValueType::Struct(_) => self.struct_ptr_type().fn_type(&param_types, false),
            ValueType::Error { .. } => self.error_ptr_type().fn_type(&param_types, false),
            ValueType::Function(_, _) => self.closure_ptr_type().fn_type(&param_types, false),
            ValueType::Optional(_) => self.value_type().fn_type(&param_types, false),
        };

        let lambda_fn = self
            .module
            .add_function(&fn_name, fn_type, Some(Linkage::Internal));

        let entry = self.context.append_basic_block(lambda_fn, "entry");
        self.builder.position_at_end(entry);
        self.push_function_return(signature.return_type.clone());
        self.push_error_mode(ErrorHandlingMode::Propagate);

        let closure_param = lambda_fn
            .get_nth_param(0)
            .ok_or_else(|| anyhow!("missing closure parameter"))?
            .into_pointer_value();

        let mut lambda_locals: HashMap<String, LocalVariable<'ctx>> = HashMap::new();

        let get_fn = self.ensure_closure_get();
        for (index, (name, capture_type)) in
            capture_names.iter().zip(capture_types.iter()).enumerate()
        {
            let tea_value = self
                .call_function(
                    get_fn,
                    &[
                        closure_param.into(),
                        self.int_type().const_int(index as u64, false).into(),
                    ],
                    &format!("get_capture_{index}"),
                )?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue from capture"))?
                .into_struct_value();
            let expr = self.tea_value_to_expr(tea_value, capture_type.clone())?;
            let alloca =
                self.create_entry_alloca(lambda_fn, name, self.basic_type(capture_type)?)?;
            if let Some(basic) = expr.into_basic_value() {
                map_builder_error(self.builder.build_store(alloca, basic))?;
            }
            lambda_locals.insert(
                name.clone(),
                LocalVariable {
                    pointer: alloca,
                    ty: capture_type.clone(),
                    mutable: true,
                },
            );
        }

        for (index, (parameter, param_type)) in lambda
            .parameters
            .iter()
            .zip(signature.param_types.iter())
            .enumerate()
        {
            let arg = lambda_fn
                .get_nth_param((index + 1) as u32)
                .ok_or_else(|| anyhow!("missing lambda parameter"))?;
            arg.set_name(&parameter.name);
            let alloca =
                self.create_entry_alloca(lambda_fn, &parameter.name, self.basic_type(param_type)?)?;
            map_builder_error(self.builder.build_store(alloca, arg))?;
            lambda_locals.insert(
                parameter.name.clone(),
                LocalVariable {
                    pointer: alloca,
                    ty: param_type.clone(),
                    mutable: true,
                },
            );
        }

        let mut locals = lambda_locals;
        match &lambda.body {
            LambdaBody::Expression(expr) => {
                let value = self.compile_expression(expr, lambda_fn, &mut locals)?;
                if matches!(signature.return_type, ValueType::Void) {
                    self.clear_error_state()?;
                    map_builder_error(self.builder.build_return(None))?;
                } else {
                    let converted = self.convert_expr_to_type(value, &signature.return_type)?;
                    self.emit_return_value(converted, &signature.return_type)?;
                }
            }
            LambdaBody::Block(block) => {
                let terminated = self.compile_block(
                    &block.statements,
                    lambda_fn,
                    &mut locals,
                    &signature.return_type,
                    true,
                )?;
                if !terminated {
                    match signature.return_type {
                        ValueType::Void => {
                            self.clear_error_state()?;
                            map_builder_error(self.builder.build_return(None))?;
                        }
                        _ => bail!("lambda is missing a return value"),
                    }
                }
            }
        }

        self.pop_error_mode();
        self.pop_function_return();
        self.lambda_functions.insert(lambda.id, lambda_fn);
        Ok(lambda_fn)
    }

    fn compile_literal(&mut self, literal: &Literal) -> Result<ExprValue<'ctx>> {
        match literal {
            Literal::Integer(value) => Ok(ExprValue::Int(
                self.int_type().const_int(*value as u64, true),
            )),
            Literal::Float(value) => Ok(ExprValue::Float(self.float_type().const_float(*value))),
            Literal::Boolean(value) => Ok(ExprValue::Bool(
                self.bool_type()
                    .const_int(if *value { 1 } else { 0 }, false),
            )),
            Literal::Nil => Ok(ExprValue::Void),
            Literal::String(value) => self.compile_string_literal(value),
        }
    }

    fn compile_string_literal(&mut self, value: &str) -> Result<ExprValue<'ctx>> {
        let bytes = value.as_bytes();
        let total_len = bytes.len() + 1;
        let array_type = self.context.i8_type().array_type(total_len as u32);
        let mut elems = Vec::with_capacity(total_len);
        for byte in bytes {
            elems.push(self.context.i8_type().const_int(*byte as u64, false));
        }
        elems.push(self.context.i8_type().const_zero());
        let const_array = self.context.i8_type().const_array(&elems);

        let name = format!(".str.{}", self.string_counter);
        self.string_counter += 1;
        let global = self.module.add_global(array_type, None, &name);
        global.set_initializer(&const_array);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);

        let zero = self.context.i32_type().const_zero();
        let ptr = unsafe {
            map_builder_error(self.builder.build_in_bounds_gep(
                array_type,
                global.as_pointer_value(),
                &[zero, zero],
                "strptr",
            ))?
        };

        let alloc_fn = self.ensure_alloc_string();
        let len_value = self.int_type().const_int(bytes.len() as u64, false);
        let call = self.call_function(alloc_fn, &[ptr.into(), len_value.into()], "alloc_str")?;
        let pointer = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected string pointer"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_interpolated_string(
        &mut self,
        template: &InterpolatedStringExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if template.parts.is_empty() {
            return self.compile_string_literal("");
        }

        let mut current: Option<PointerValue<'ctx>> = None;

        for part in &template.parts {
            let part_ptr = match part {
                InterpolatedStringPart::Literal(text) => {
                    self.compile_string_literal(text)?.into_string()?
                }
                InterpolatedStringPart::Expression(expr) => {
                    let value = self.compile_expression(expr, function, locals)?;
                    self.expr_to_string_pointer(value)?
                }
            };

            current = Some(match current {
                Some(existing) => self.concat_string_values(existing, part_ptr)?,
                None => part_ptr,
            });
        }

        let pointer =
            current.ok_or_else(|| anyhow::anyhow!("interpolated string evaluation failed"))?;
        Ok(ExprValue::String(pointer))
    }

    fn compile_binary(
        &mut self,
        expression: &BinaryExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match expression.operator {
            BinaryOperator::And => {
                return self.build_logical_and(
                    &expression.left,
                    &expression.right,
                    function,
                    locals,
                );
            }
            BinaryOperator::Or => {
                return self.build_logical_or(
                    &expression.left,
                    &expression.right,
                    function,
                    locals,
                );
            }
            BinaryOperator::Coalesce => {
                return self.build_coalesce(&expression.left, &expression.right, function, locals);
            }
            _ => {}
        }

        let left = self.compile_expression(&expression.left, function, locals)?;
        let right = self.compile_expression(&expression.right, function, locals)?;

        match expression.operator {
            BinaryOperator::Add => self.build_numeric_add(left, right),
            BinaryOperator::Subtract => self.build_numeric_sub(left, right),
            BinaryOperator::Multiply => self.build_numeric_mul(left, right),
            BinaryOperator::Divide => self.build_numeric_div(left, right),
            BinaryOperator::Modulo => self.build_numeric_mod(left, right),
            BinaryOperator::Equal => self.build_equality(function, left, right, true),
            BinaryOperator::NotEqual => self.build_equality(function, left, right, false),
            BinaryOperator::Greater => {
                self.build_numeric_compare(left, right, IntPredicate::SGT, FloatPredicate::OGT)
            }
            BinaryOperator::GreaterEqual => {
                self.build_numeric_compare(left, right, IntPredicate::SGE, FloatPredicate::OGE)
            }
            BinaryOperator::Less => {
                self.build_numeric_compare(left, right, IntPredicate::SLT, FloatPredicate::OLT)
            }
            BinaryOperator::LessEqual => {
                self.build_numeric_compare(left, right, IntPredicate::SLE, FloatPredicate::OLE)
            }
            BinaryOperator::And | BinaryOperator::Or => unreachable!(),
            BinaryOperator::Coalesce => unreachable!(),
        }
    }

    fn compile_expression(
        &mut self,
        expression: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match &expression.kind {
            ExpressionKind::Literal(lit) => self.compile_literal(lit),
            ExpressionKind::InterpolatedString(template) => {
                self.compile_interpolated_string(template, function, locals)
            }
            ExpressionKind::Identifier(ident) => {
                if let Some(variable) = locals.get(ident.name.as_str()) {
                    self.load_local_variable(&ident.name, variable)
                } else if let Some(slot) = self.global_slots.get(ident.name.as_str()).cloned() {
                    self.load_global_variable(&ident.name, slot)
                } else {
                    Err(self.undefined_identifier_error(&ident.name, expression.span))
                }
            }
            ExpressionKind::Binary(binary) => self.compile_binary(binary, function, locals),
            ExpressionKind::Unary(unary) => {
                let operand = self.compile_expression(&unary.operand, function, locals)?;
                match unary.operator {
                    crate::ast::UnaryOperator::Positive => Ok(operand),
                    crate::ast::UnaryOperator::Negative => match operand {
                        ExprValue::Int(value) => Ok(ExprValue::Int(map_builder_error(
                            self.builder.build_int_neg(value, "negtmp"),
                        )?)),
                        ExprValue::Float(value) => Ok(ExprValue::Float(map_builder_error(
                            self.builder.build_float_neg(value, "fnegtmp"),
                        )?)),
                        _ => bail!("unary '-' expects numeric operand"),
                    },
                    crate::ast::UnaryOperator::Not => {
                        let bool_value = operand.into_bool()?;
                        let not_value =
                            map_builder_error(self.builder.build_not(bool_value, "nottmp"))?;
                        Ok(ExprValue::Bool(not_value))
                    }
                }
            }
            ExpressionKind::Is(_) => bail!("'is' expressions are not supported in LLVM backend"),
            ExpressionKind::Call(call) => {
                self.compile_call(call, expression.span, function, locals)
            }
            ExpressionKind::Grouping(inner) => self.compile_expression(inner, function, locals),
            ExpressionKind::Assignment(assign) => self.compile_assignment(assign, function, locals),
            ExpressionKind::List(list_literal) => {
                self.compile_list_literal(list_literal, function, locals)
            }
            ExpressionKind::Dict(dict_literal) => {
                self.compile_dict_literal(dict_literal, function, locals)
            }
            ExpressionKind::Index(index_expr) => self.compile_index(index_expr, function, locals),
            ExpressionKind::Member(member_expr) => {
                self.compile_member(member_expr, function, locals)
            }
            ExpressionKind::Lambda(lambda) => {
                self.compile_lambda_expression(lambda, function, locals)
            }
            ExpressionKind::Match(_) => bail!("unsupported expression"),
            ExpressionKind::Unwrap(inner) => {
                let value = self.compile_expression(inner, function, locals)?;
                match value {
                    ExprValue::Optional { value, inner } => {
                        self.compile_optional_unwrap(value, inner, function)
                    }
                    ExprValue::Void => {
                        bail!("cannot unwrap nil value")
                    }
                    _ => bail!("unwrap expects an optional value"),
                }
            }
            ExpressionKind::Try(try_expr) => {
                self.compile_try_expression(try_expr, function, locals)
            }
            ExpressionKind::Range(_) => bail!("unsupported expression"),
        }
    }

    fn load_local_variable(
        &mut self,
        name: &str,
        variable: &LocalVariable<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        self.load_from_pointer(variable.pointer, &variable.ty, name)
    }

    fn load_from_pointer(
        &mut self,
        pointer: PointerValue<'ctx>,
        ty: &ValueType,
        name: &str,
    ) -> Result<ExprValue<'ctx>> {
        match ty {
            ValueType::Int => {
                let loaded =
                    map_builder_error(self.builder.build_load(self.int_type(), pointer, name))?;
                Ok(ExprValue::Int(loaded.into_int_value()))
            }
            ValueType::Float => {
                let loaded =
                    map_builder_error(self.builder.build_load(self.float_type(), pointer, name))?;
                Ok(ExprValue::Float(loaded.into_float_value()))
            }
            ValueType::Bool => {
                let loaded =
                    map_builder_error(self.builder.build_load(self.bool_type(), pointer, name))?;
                Ok(ExprValue::Bool(loaded.into_int_value()))
            }
            ValueType::String => {
                let loaded = map_builder_error(self.builder.build_load(
                    self.string_ptr_type(),
                    pointer,
                    name,
                ))?;
                Ok(ExprValue::String(loaded.into_pointer_value()))
            }
            ValueType::List(element_type) => {
                let loaded = map_builder_error(self.builder.build_load(
                    self.list_ptr_type(),
                    pointer,
                    name,
                ))?;
                Ok(ExprValue::List {
                    pointer: loaded.into_pointer_value(),
                    element_type: element_type.clone(),
                })
            }
            ValueType::Dict(value_type) => {
                let loaded = map_builder_error(self.builder.build_load(
                    self.dict_ptr_type(),
                    pointer,
                    name,
                ))?;
                Ok(ExprValue::Dict {
                    pointer: loaded.into_pointer_value(),
                    value_type: value_type.clone(),
                })
            }
            ValueType::Struct(struct_name) => {
                let loaded = map_builder_error(self.builder.build_load(
                    self.struct_ptr_type(),
                    pointer,
                    name,
                ))?;
                Ok(ExprValue::Struct {
                    pointer: loaded.into_pointer_value(),
                    struct_name: struct_name.clone(),
                })
            }
            ValueType::Error {
                error_name,
                variant_name,
            } => {
                let loaded = map_builder_error(self.builder.build_load(
                    self.error_ptr_type(),
                    pointer,
                    name,
                ))?;
                Ok(ExprValue::Error {
                    pointer: loaded.into_pointer_value(),
                    error_name: error_name.clone(),
                    variant_name: variant_name.clone(),
                })
            }
            ValueType::Function(param_types, return_type) => {
                let loaded = map_builder_error(self.builder.build_load(
                    self.closure_ptr_type(),
                    pointer,
                    name,
                ))?;
                Ok(ExprValue::Closure {
                    pointer: loaded.into_pointer_value(),
                    param_types: param_types.clone(),
                    return_type: return_type.clone(),
                })
            }
            ValueType::Optional(inner) => {
                let loaded =
                    map_builder_error(self.builder.build_load(self.value_type(), pointer, name))?;
                Ok(ExprValue::Optional {
                    value: loaded.into_struct_value(),
                    inner: inner.clone(),
                })
            }
            ValueType::Void => bail!("void value cannot be loaded"),
        }
    }

    fn load_global_variable(
        &mut self,
        name: &str,
        slot: GlobalBindingSlot<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        self.load_from_pointer(slot.pointer.as_pointer_value(), &slot.ty, name)
    }

    fn describe_span(span: SourceSpan) -> String {
        if span.line == 0 {
            "unknown location".to_string()
        } else if span.line == span.end_line && span.column == span.end_column {
            format!("line {}, column {}", span.line, span.column)
        } else {
            format!(
                "line {} column {} to line {} column {}",
                span.line, span.column, span.end_line, span.end_column
            )
        }
    }

    fn undefined_identifier_error(&self, name: &str, span: SourceSpan) -> anyhow::Error {
        anyhow!(format!(
            "undefined identifier '{}' at {}",
            name,
            Self::describe_span(span)
        ))
    }

    fn compile_call(
        &mut self,
        call: &CallExpression,
        span: SourceSpan,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if let ExpressionKind::Member(member) = &call.callee.kind {
            if let ExpressionKind::Identifier(alias_ident) = &member.object.kind {
                if let Some(functions) = self.module_builtins.get(&alias_ident.name) {
                    if let Some(&kind) = functions.get(&member.property) {
                        return self.compile_builtin_call(kind, call, function, locals);
                    }
                }
            }
        }

        if let ExpressionKind::Identifier(identifier) = &call.callee.kind {
            if let Some(kind) = self
                .builtin_functions
                .get(identifier.name.as_str())
                .copied()
            {
                return self.compile_builtin_call(kind, call, function, locals);
            }

            if let Some(expr) = self.try_compile_error_constructor(
                &identifier.name,
                None,
                call,
                span,
                function,
                locals,
            )? {
                return Ok(expr);
            }

            if !locals.contains_key(identifier.name.as_str()) {
                let (target_name, display_name) = if let Some((_, instance)) =
                    self.function_call_metadata_tc.get(&span)
                {
                    let mangled = mangle_function_name(&identifier.name, &instance.type_arguments);
                    (mangled, identifier.name.clone())
                } else {
                    (identifier.name.clone(), identifier.name.clone())
                };
                if let Some(signature) = self.functions.get(&target_name).cloned() {
                    if signature.param_types.len() != call.arguments.len() {
                        bail!(
                            "call to '{}' expects {} arguments, found {}",
                            display_name,
                            signature.param_types.len(),
                            call.arguments.len()
                        );
                    }

                    let mut args = Vec::with_capacity(call.arguments.len());
                    for (index, argument) in call.arguments.iter().enumerate() {
                        if argument.name.is_some() {
                            bail!("named arguments are not supported by the LLVM backend yet");
                        }
                        let value =
                            self.compile_expression(&argument.expression, function, locals)?;
                        let expected = &signature.param_types[index];
                        let converted =
                            self.convert_expr_to_type(value, expected)
                                .map_err(|error| {
                                    anyhow!(
                                        "argument {} to '{}' has mismatched type: {}",
                                        index + 1,
                                        display_name,
                                        error
                                    )
                                })?;
                        let basic = converted
                            .into_basic_value()
                            .ok_or_else(|| anyhow!("argument must produce a value"))?;
                        args.push(basic.into());
                    }

                    let call_site = self.call_function(signature.value, &args, &target_name)?;
                    if matches!(signature.return_type, ValueType::Void) {
                        if signature.can_throw {
                            self.handle_possible_error(function)?;
                        }
                        return Ok(ExprValue::Void);
                    }

                    let result = call_site.try_as_basic_value().left().ok_or_else(|| {
                        anyhow!(format!("function '{}' returned no value", display_name))
                    })?;

                    let expr = match signature.return_type {
                        ValueType::Int => ExprValue::Int(result.into_int_value()),
                        ValueType::Float => ExprValue::Float(result.into_float_value()),
                        ValueType::Bool => ExprValue::Bool(result.into_int_value()),
                        ValueType::String => ExprValue::String(result.into_pointer_value()),
                        ValueType::List(inner) => ExprValue::List {
                            pointer: result.into_pointer_value(),
                            element_type: inner,
                        },
                        ValueType::Dict(inner) => ExprValue::Dict {
                            pointer: result.into_pointer_value(),
                            value_type: inner,
                        },
                        ValueType::Struct(struct_name) => ExprValue::Struct {
                            pointer: result.into_pointer_value(),
                            struct_name,
                        },
                        ValueType::Error {
                            error_name,
                            variant_name,
                        } => ExprValue::Error {
                            pointer: result.into_pointer_value(),
                            error_name,
                            variant_name,
                        },
                        ValueType::Function(params, ret) => ExprValue::Closure {
                            pointer: result.into_pointer_value(),
                            param_types: params,
                            return_type: ret,
                        },
                        ValueType::Optional(inner) => ExprValue::Optional {
                            value: result.into_struct_value(),
                            inner,
                        },
                        ValueType::Void => unreachable!(),
                    };
                    if signature.can_throw {
                        self.handle_possible_error(function)?;
                    }
                    return Ok(expr);
                }

                if self.structs.contains_key(&identifier.name) {
                    return self.compile_struct_constructor(
                        &identifier.name,
                        span,
                        &call.arguments,
                        function,
                        locals,
                    );
                }

                let has_variable = locals.contains_key(identifier.name.as_str())
                    || self.global_slots.contains_key(identifier.name.as_str());
                if !has_variable {
                    bail!(format!("undefined function '{}'", identifier.name));
                }
            }
        }

        if let ExpressionKind::Member(member) = &call.callee.kind {
            if let ExpressionKind::Identifier(base) = &member.object.kind {
                if let Some(expr) = self.try_compile_error_constructor(
                    &base.name,
                    Some(&member.property),
                    call,
                    span,
                    function,
                    locals,
                )? {
                    return Ok(expr);
                }
            }
        }

        let callee_value = self.compile_expression(&call.callee, function, locals)?;
        if let ExprValue::Closure {
            pointer,
            param_types,
            return_type,
        } = callee_value
        {
            return self.call_closure(
                pointer,
                &param_types,
                return_type.as_ref(),
                call,
                function,
                locals,
            );
        }

        bail!("unsupported call target in LLVM backend");
    }

    fn compile_builtin_call(
        &mut self,
        kind: StdFunctionKind,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match kind {
            StdFunctionKind::Print => self.compile_print_call(&call.arguments, function, locals),
            StdFunctionKind::Assert => self.compile_assert_call(&call.arguments, function, locals),
            StdFunctionKind::AssertEq => {
                self.compile_assert_eq_call(&call.arguments, function, locals)
            }
            StdFunctionKind::AssertNe => {
                self.compile_assert_ne_call(&call.arguments, function, locals)
            }
            StdFunctionKind::AssertFail => {
                self.compile_fail_call(&call.arguments, function, locals)
            }
            StdFunctionKind::AssertSnapshot => {
                bail!("assert_snapshot is not supported by the LLVM backend yet")
            }
            StdFunctionKind::AssertEmpty => {
                bail!("assert_empty is not supported by the LLVM backend yet")
            }
            StdFunctionKind::UtilLen => {
                self.compile_util_len_call(&call.arguments, function, locals)
            }
            StdFunctionKind::UtilToString => {
                self.compile_util_to_string_call(&call.arguments, function, locals)
            }
            StdFunctionKind::UtilClampInt => {
                self.compile_util_clamp_int_call(&call.arguments, function, locals)
            }
            StdFunctionKind::UtilIsNil
            | StdFunctionKind::UtilIsBool
            | StdFunctionKind::UtilIsInt
            | StdFunctionKind::UtilIsFloat
            | StdFunctionKind::UtilIsString
            | StdFunctionKind::UtilIsList
            | StdFunctionKind::UtilIsStruct
            | StdFunctionKind::UtilIsError => {
                self.compile_util_predicate_call(&call.arguments, function, locals, kind)
            }
            StdFunctionKind::EnvGet => self.compile_env_get_call(&call.arguments, function, locals),
            StdFunctionKind::EnvGetOr => {
                self.compile_env_get_or_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvHas => self.compile_env_has_call(&call.arguments, function, locals),
            StdFunctionKind::EnvRequire => {
                self.compile_env_require_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvSet => self.compile_env_set_call(&call.arguments, function, locals),
            StdFunctionKind::EnvUnset => {
                self.compile_env_unset_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvVars => {
                self.compile_env_vars_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvCwd => self.compile_env_cwd_call(&call.arguments, function, locals),
            StdFunctionKind::EnvSetCwd => {
                self.compile_env_set_cwd_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvTempDir => {
                self.compile_env_temp_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvHomeDir => {
                self.compile_env_home_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvConfigDir => {
                self.compile_env_config_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathJoin => {
                self.compile_path_join_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathComponents => {
                self.compile_path_components_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathDirname => {
                self.compile_path_dirname_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathBasename => {
                self.compile_path_basename_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathExtension => {
                self.compile_path_extension_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathSetExtension => {
                self.compile_path_set_extension_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathStripExtension => {
                self.compile_path_strip_extension_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathNormalize => {
                self.compile_path_normalize_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathAbsolute => {
                self.compile_path_absolute_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathRelative => {
                self.compile_path_relative_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathIsAbsolute => {
                self.compile_path_is_absolute_call(&call.arguments, function, locals)
            }
            StdFunctionKind::PathSeparator => self.compile_path_separator_call(&call.arguments),
            StdFunctionKind::IoReadLine => {
                self.compile_io_read_line_call(&call.arguments, function, locals)
            }
            StdFunctionKind::IoReadAll => self.compile_io_read_all_call(&call.arguments, locals),
            StdFunctionKind::IoReadBytes => {
                self.compile_io_read_bytes_call(&call.arguments, locals)
            }
            StdFunctionKind::IoWrite => {
                self.compile_io_write_call(&call.arguments, function, locals)
            }
            StdFunctionKind::IoWriteErr => {
                self.compile_io_write_err_call(&call.arguments, function, locals)
            }
            StdFunctionKind::IoFlush => self.compile_io_flush_call(&call.arguments),
            StdFunctionKind::JsonEncode => {
                self.compile_json_encode_call(&call.arguments, function, locals)
            }
            StdFunctionKind::JsonDecode => {
                self.compile_json_decode_call(&call.arguments, function, locals)
            }
            StdFunctionKind::YamlEncode => {
                self.compile_yaml_encode_call(&call.arguments, function, locals)
            }
            StdFunctionKind::YamlDecode => {
                self.compile_yaml_decode_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsReadText => {
                self.compile_fs_read_text_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsWriteText => {
                self.compile_fs_write_text_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsWriteTextAtomic => {
                self.compile_fs_write_text_atomic_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsReadBytes => {
                self.compile_fs_read_bytes_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsWriteBytes => {
                self.compile_fs_write_bytes_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsWriteBytesAtomic => {
                self.compile_fs_write_bytes_atomic_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsCreateDir => {
                self.compile_fs_create_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsEnsureDir => {
                self.compile_fs_ensure_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsEnsureParent => {
                self.compile_fs_ensure_parent_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsRemove => {
                self.compile_fs_remove_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsExists => {
                self.compile_fs_exists_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsIsDir => {
                self.compile_fs_is_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsIsSymlink => {
                self.compile_fs_is_symlink_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsListDir => {
                self.compile_fs_list_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsWalk => self.compile_fs_walk_call(&call.arguments, function, locals),
            StdFunctionKind::FsGlob => self.compile_fs_glob_call(&call.arguments, function, locals),
            StdFunctionKind::FsSize => self.compile_fs_size_call(&call.arguments, function, locals),
            StdFunctionKind::FsModified => {
                self.compile_fs_modified_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsPermissions => {
                self.compile_fs_permissions_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsIsReadonly => {
                self.compile_fs_is_readonly_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsMetadata => {
                self.compile_fs_metadata_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsOpenRead => {
                self.compile_fs_open_read_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsReadChunk => {
                self.compile_fs_read_chunk_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsClose => {
                self.compile_fs_close_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessRun => {
                self.compile_process_run_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessSpawn => {
                self.compile_process_spawn_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessWait => {
                self.compile_process_wait_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessKill => {
                self.compile_process_kill_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessReadStdout => {
                self.compile_process_read_call(&call.arguments, function, locals, true)
            }
            StdFunctionKind::ProcessReadStderr => {
                self.compile_process_read_call(&call.arguments, function, locals, false)
            }
            StdFunctionKind::ProcessWriteStdin => {
                self.compile_process_write_stdin_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessCloseStdin => {
                self.compile_process_close_stdin_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessClose => {
                self.compile_process_close_call(&call.arguments, function, locals)
            }
            StdFunctionKind::CliArgs => {
                self.compile_cli_args_call(&call.arguments, function, locals)
            }
            StdFunctionKind::CliParse => {
                self.compile_cli_parse_call(&call.arguments, function, locals)
            }
            StdFunctionKind::CliCapture => {
                bail!("support.cli.capture is not supported by the LLVM backend yet")
            }
        }
    }
    fn compile_assert_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !(1..=2).contains(&arguments.len()) {
            bail!("assert expects 1 or 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for assert");
            }
        }

        let condition = self
            .compile_expression(&arguments[0].expression, function, locals)?
            .into_bool()?;
        let condition_i32 = self.bool_to_i32(condition, "assert_cond")?;

        let message_ptr = if arguments.len() == 2 {
            let message_value =
                self.compile_expression(&arguments[1].expression, function, locals)?;
            match message_value {
                ExprValue::String(ptr) => ptr,
                _ => bail!("assert message must be a String"),
            }
        } else {
            self.string_ptr_type().const_null()
        };

        let func = self.ensure_assert_fn();
        self.call_function(
            func,
            &[condition_i32.into(), message_ptr.into()],
            "tea_assert",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_assert_eq_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("assert_eq expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for assert_eq");
            }
        }

        let left_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let right_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let left_value = self.expr_to_tea_value(left_expr)?;
        let right_value = self.expr_to_tea_value(right_expr)?;

        let func = self.ensure_assert_eq_fn();
        self.call_function(
            func,
            &[left_value.into(), right_value.into()],
            "tea_assert_eq",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_assert_ne_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("assert_ne expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for assert_ne");
            }
        }

        let left_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let right_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let left_value = self.expr_to_tea_value(left_expr)?;
        let right_value = self.expr_to_tea_value(right_expr)?;

        let func = self.ensure_assert_ne_fn();
        self.call_function(
            func,
            &[left_value.into(), right_value.into()],
            "tea_assert_ne",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_fail_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("fail expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for fail");
        }

        let message_value = self.compile_expression(&arguments[0].expression, function, locals)?;
        let message_ptr = match message_value {
            ExprValue::String(ptr) => ptr,
            _ => bail!("fail message must be a String"),
        };
        let func = self.ensure_fail_fn();
        self.call_function(func, &[message_ptr.into()], "tea_fail")?;
        Ok(ExprValue::Void)
    }

    fn compile_util_len_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("len expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for len");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let tea_value = self.expr_to_tea_value(value_expr)?;
        let func = self.ensure_util_len_fn();
        let length = self
            .call_function(func, &[tea_value.into()], "tea_util_len")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_util_len returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(length))
    }

    fn compile_util_to_string_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("to_string expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for to_string");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let tea_value = self.expr_to_tea_value(value_expr)?;
        let func = self.ensure_util_to_string_fn();
        let pointer = self
            .call_function(func, &[tea_value.into()], "tea_util_to_string")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_util_to_string returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_util_clamp_int_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 3 {
            bail!("clamp_int expects 3 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for clamp_int");
            }
        }
        let value = self
            .compile_expression(&arguments[0].expression, function, locals)?
            .into_int()?;
        let min_value = self
            .compile_expression(&arguments[1].expression, function, locals)?
            .into_int()?;
        let max_value = self
            .compile_expression(&arguments[2].expression, function, locals)?
            .into_int()?;

        let func = self.ensure_util_clamp_int_fn();
        let result = self
            .call_function(
                func,
                &[value.into(), min_value.into(), max_value.into()],
                "tea_util_clamp_int",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_util_clamp_int returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(result))
    }

    fn compile_util_predicate_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        kind: StdFunctionKind,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("type guard expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for type guard");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let tea_value = self.expr_to_tea_value(value_expr)?;
        let (func, label) = match kind {
            StdFunctionKind::UtilIsNil => (self.ensure_util_is_nil_fn(), "tea_util_is_nil"),
            StdFunctionKind::UtilIsBool => (self.ensure_util_is_bool_fn(), "tea_util_is_bool"),
            StdFunctionKind::UtilIsInt => (self.ensure_util_is_int_fn(), "tea_util_is_int"),
            StdFunctionKind::UtilIsFloat => (self.ensure_util_is_float_fn(), "tea_util_is_float"),
            StdFunctionKind::UtilIsString => {
                (self.ensure_util_is_string_fn(), "tea_util_is_string")
            }
            StdFunctionKind::UtilIsList => (self.ensure_util_is_list_fn(), "tea_util_is_list"),
            StdFunctionKind::UtilIsStruct => {
                (self.ensure_util_is_struct_fn(), "tea_util_is_struct")
            }
            StdFunctionKind::UtilIsError => (self.ensure_util_is_error_fn(), "tea_util_is_error"),
            _ => bail!("unsupported util predicate"),
        };
        let raw = self
            .call_function(func, &[tea_value.into()], label)?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!(format!("{label} returned no value")))?
            .into_int_value();
        let bool_val = self.i32_to_bool(raw, &(label.to_string() + "_bool"))?;
        Ok(ExprValue::Bool(bool_val))
    }

    fn compile_env_get_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("env.get expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for env.get");
        }
        let name_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let name_ptr = match name_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.get expects the name argument to be a String"),
        };
        let func = self.ensure_env_get_fn();
        let pointer = self
            .call_function(func, &[name_ptr.into()], "tea_env_get")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_get returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_env_get_or_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("env.get_or expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for env.get_or");
            }
        }
        let name_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let name_ptr = match name_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.get_or expects the name argument to be a String"),
        };
        let fallback_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let fallback_ptr = match fallback_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.get_or expects the fallback argument to be a String"),
        };
        let func = self.ensure_env_get_or_fn();
        let pointer = self
            .call_function(
                func,
                &[name_ptr.into(), fallback_ptr.into()],
                "tea_env_get_or",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_get_or returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_env_has_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("env.has expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for env.has");
        }
        let name_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let name_ptr = match name_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.has expects the name argument to be a String"),
        };
        let func = self.ensure_env_has_fn();
        let raw = self
            .call_function(func, &[name_ptr.into()], "tea_env_has")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_has returned no value"))?
            .into_int_value();
        let bool_val = self.i32_to_bool(raw, "tea_env_has_bool")?;
        Ok(ExprValue::Bool(bool_val))
    }

    fn compile_env_require_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("env.require expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for env.require");
        }
        let name_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let name_ptr = match name_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.require expects the name argument to be a String"),
        };
        let func = self.ensure_env_require_fn();
        let pointer = self
            .call_function(func, &[name_ptr.into()], "tea_env_require")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_require returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_env_set_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("env.set expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for env.set");
            }
        }
        let name_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let name_ptr = match name_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.set expects the name argument to be a String"),
        };
        let value_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let value_ptr = match value_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.set expects the value argument to be a String"),
        };
        let func = self.ensure_env_set_fn();
        self.call_function(func, &[name_ptr.into(), value_ptr.into()], "tea_env_set")?;
        Ok(ExprValue::Void)
    }

    fn compile_env_unset_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("env.unset expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for env.unset");
        }
        let name_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let name_ptr = match name_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.unset expects the name argument to be a String"),
        };
        let func = self.ensure_env_unset_fn();
        self.call_function(func, &[name_ptr.into()], "tea_env_unset")?;
        Ok(ExprValue::Void)
    }

    fn compile_env_vars_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("env.vars expects no arguments");
        }
        let func = self.ensure_env_vars_fn();
        let value = self
            .call_function(func, &[], "tea_env_vars")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_vars returned no value"))?
            .into_struct_value();
        self.tea_value_to_expr(value, ValueType::Dict(Box::new(ValueType::String)))
    }

    fn compile_env_cwd_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("env.cwd expects no arguments");
        }
        let func = self.ensure_env_cwd_fn();
        let pointer = self
            .call_function(func, &[], "tea_env_cwd")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_cwd returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_env_set_cwd_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("env.set_cwd expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for env.set_cwd");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("env.set_cwd expects the path argument to be a String"),
        };
        let func = self.ensure_env_set_cwd_fn();
        self.call_function(func, &[path_ptr.into()], "tea_env_set_cwd")?;
        Ok(ExprValue::Void)
    }

    fn compile_env_temp_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("env.temp_dir expects no arguments");
        }
        let func = self.ensure_env_temp_dir_fn();
        let pointer = self
            .call_function(func, &[], "tea_env_temp_dir")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_temp_dir returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_env_home_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("env.home_dir expects no arguments");
        }
        let func = self.ensure_env_home_dir_fn();
        let pointer = self
            .call_function(func, &[], "tea_env_home_dir")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_home_dir returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_env_config_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("env.config_dir expects no arguments");
        }
        let func = self.ensure_env_config_dir_fn();
        let pointer = self
            .call_function(func, &[], "tea_env_config_dir")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_env_config_dir returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_join_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.join expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.join");
        }
        let list_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let list_ptr = match list_expr {
            ExprValue::List { pointer, .. } => pointer,
            _ => bail!("path.join expects a List argument"),
        };
        let func = self.ensure_path_join_fn();
        let pointer = self
            .call_function(func, &[list_ptr.into()], "tea_path_join")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_join returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_components_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.components expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.components");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.components expects the argument to be a String"),
        };
        let func = self.ensure_path_components_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_path_components")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_components returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_path_dirname_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.dirname expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.dirname");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.dirname expects the argument to be a String"),
        };
        let func = self.ensure_path_dirname_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_path_dirname")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_dirname returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_basename_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.basename expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.basename");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.basename expects the argument to be a String"),
        };
        let func = self.ensure_path_basename_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_path_basename")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_basename returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_extension_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.extension expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.extension");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.extension expects the argument to be a String"),
        };
        let func = self.ensure_path_extension_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_path_extension")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_extension returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_set_extension_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("path.set_extension expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for path.set_extension");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.set_extension expects the path argument to be a String"),
        };
        let ext_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let ext_ptr = match ext_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.set_extension expects the extension argument to be a String"),
        };
        let func = self.ensure_path_set_extension_fn();
        let pointer = self
            .call_function(
                func,
                &[path_ptr.into(), ext_ptr.into()],
                "tea_path_set_extension",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_set_extension returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_strip_extension_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.strip_extension expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.strip_extension");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.strip_extension expects the argument to be a String"),
        };
        let func = self.ensure_path_strip_extension_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_path_strip_extension")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_strip_extension returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_normalize_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.normalize expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.normalize");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.normalize expects the argument to be a String"),
        };
        let func = self.ensure_path_normalize_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_path_normalize")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_normalize returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_absolute_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !(1..=2).contains(&arguments.len()) {
            bail!("path.absolute expects 1 or 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for path.absolute");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.absolute expects the path argument to be a String"),
        };
        let (base_ptr, has_base) = if arguments.len() == 2 {
            let base_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            let base_ptr = match base_expr {
                ExprValue::String(ptr) => ptr,
                _ => bail!("path.absolute expects the base argument to be a String"),
            };
            let flag = self.bool_to_i32(self.bool_type().const_all_ones(), "path_absolute_has")?;
            (base_ptr, flag)
        } else {
            let null_ptr = self.string_ptr_type().const_null();
            let flag = self.bool_to_i32(self.bool_type().const_zero(), "path_absolute_none")?;
            (null_ptr, flag)
        };
        let func = self.ensure_path_absolute_fn();
        let pointer = self
            .call_function(
                func,
                &[path_ptr.into(), base_ptr.into(), has_base.into()],
                "tea_path_absolute",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_absolute returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_relative_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("path.relative expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for path.relative");
            }
        }
        let target_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let target_ptr = match target_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.relative expects the target argument to be a String"),
        };
        let base_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let base_ptr = match base_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.relative expects the base argument to be a String"),
        };
        let func = self.ensure_path_relative_fn();
        let pointer = self
            .call_function(
                func,
                &[target_ptr.into(), base_ptr.into()],
                "tea_path_relative",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_relative returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_path_is_absolute_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("path.is_absolute expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for path.is_absolute");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("path.is_absolute expects the argument to be a String"),
        };
        let func = self.ensure_path_is_absolute_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_path_is_absolute")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_is_absolute returned no value"))?
            .into_int_value();
        let bool_val = self.i32_to_bool(raw, "tea_path_is_absolute_bool")?;
        Ok(ExprValue::Bool(bool_val))
    }

    fn compile_path_separator_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("path.separator expects no arguments");
        }
        let func = self.ensure_path_separator_fn();
        let pointer = self
            .call_function(func, &[], "tea_path_separator")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_path_separator returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_io_read_line_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("read_line expects no arguments");
        }

        let value_func = self.ensure_io_read_line();
        let tea_value = self
            .call_function(value_func, &[], "tea_io_read_line")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_io_read_line returned no value"))?
            .into_struct_value();

        let is_nil_fn = self.ensure_util_is_nil_fn();
        let raw_nil = self
            .call_function(
                is_nil_fn,
                &[tea_value.as_basic_value_enum().into()],
                "tea_io_read_line_is_nil",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_io_read_line_is_nil returned no value"))?
            .into_int_value();
        let is_nil = self.i32_to_bool(raw_nil, "io_read_line_is_nil")?;

        let nil_block = self
            .context
            .append_basic_block(function, "io_read_line_nil");
        let string_block = self
            .context
            .append_basic_block(function, "io_read_line_string");
        let merge_block = self
            .context
            .append_basic_block(function, "io_read_line_merge");

        map_builder_error(
            self.builder
                .build_conditional_branch(is_nil, nil_block, string_block),
        )?;

        self.builder.position_at_end(nil_block);
        let null_ptr = self.string_ptr_type().const_null();
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
        let nil_end = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing nil block for io_read_line"))?;

        self.builder.position_at_end(string_block);
        let as_string_fn = self.ensure_value_as_string();
        let string_ptr = self
            .call_function(
                as_string_fn,
                &[tea_value.as_basic_value_enum().into()],
                "io_read_line_as_string",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_value_as_string returned no value"))?
            .into_pointer_value();
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
        let string_end = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing string block for io_read_line"))?;

        self.builder.position_at_end(merge_block);
        let phi = map_builder_error(
            self.builder
                .build_phi(self.string_ptr_type(), "io_read_line_phi"),
        )?;
        let null_basic = null_ptr.as_basic_value_enum();
        let string_basic = string_ptr.as_basic_value_enum();
        phi.add_incoming(&[(&null_basic, nil_end), (&string_basic, string_end)]);

        Ok(ExprValue::String(phi.as_basic_value().into_pointer_value()))
    }

    fn compile_io_read_all_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("read_all expects no arguments");
        }
        let func = self.ensure_io_read_all();
        let ptr = self
            .call_function(func, &[], "tea_io_read_all")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_io_read_all returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(ptr))
    }

    fn compile_io_read_bytes_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("read_bytes expects no arguments");
        }
        let func = self.ensure_io_read_bytes();
        let ptr = self
            .call_function(func, &[], "tea_io_read_bytes")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_io_read_bytes returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer: ptr,
            element_type: Box::new(ValueType::Int),
        })
    }

    fn compile_io_write_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("write expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for write");
        }
        let expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let ptr = self.expect_string_pointer(expr, "write expects a String argument")?;
        let func = self.ensure_io_write();
        self.call_function(func, &[ptr.into()], "tea_io_write")?;
        Ok(ExprValue::Void)
    }

    fn compile_io_write_err_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("write_err expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for write_err");
        }
        let expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let ptr = self.expect_string_pointer(expr, "write_err expects a String argument")?;
        let func = self.ensure_io_write_err();
        self.call_function(func, &[ptr.into()], "tea_io_write_err")?;
        Ok(ExprValue::Void)
    }

    fn compile_io_flush_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("flush expects no arguments");
        }
        let func = self.ensure_io_flush();
        self.call_function(func, &[], "tea_io_flush")?;
        Ok(ExprValue::Void)
    }

    fn compile_cli_args_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !arguments.is_empty() {
            bail!("cli.args expects no arguments");
        }
        let func = self.ensure_cli_args_fn();
        let pointer = self
            .call_function(func, &[], "tea_cli_args")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_cli_args returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_cli_parse_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.is_empty() || arguments.len() > 2 {
            bail!("cli.parse expects 1 or 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for cli.parse");
            }
        }

        let spec_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let spec_value = BasicMetadataValueEnum::from(self.expr_to_tea_value(spec_expr)?);

        let override_value = if arguments.len() == 2 {
            let value_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            BasicMetadataValueEnum::from(self.expr_to_tea_value(value_expr)?)
        } else {
            let func_nil = self.ensure_value_nil();
            let call = self.call_function(func_nil, &[], "val_nil")?;
            let nil_value = call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("tea_value_nil returned no value"))?;
            BasicMetadataValueEnum::from(nil_value)
        };

        let template_ptr = self.ensure_struct_template("CliParseResult")?;
        let func = self.ensure_cli_parse_fn();
        let pointer = self
            .call_function(
                func,
                &[template_ptr.into(), spec_value, override_value],
                "tea_cli_parse",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_cli_parse returned no value"))?
            .into_pointer_value();

        Ok(ExprValue::Struct {
            pointer,
            struct_name: "CliParseResult".to_string(),
        })
    }

    fn build_nil_value(&mut self) -> Result<BasicMetadataValueEnum<'ctx>> {
        let func_nil = self.ensure_value_nil();
        let call = self.call_function(func_nil, &[], "val_nil")?;
        let nil_value = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_value_nil returned no value"))?;
        Ok(BasicMetadataValueEnum::from(nil_value))
    }

    fn expr_to_metadata_value(
        &mut self,
        value: ExprValue<'ctx>,
    ) -> Result<BasicMetadataValueEnum<'ctx>> {
        let tea_value = self.expr_to_tea_value(value)?;
        Ok(BasicMetadataValueEnum::from(tea_value))
    }

    fn compile_process_run_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.is_empty() || arguments.len() > 5 {
            bail!("process.run expects between 1 and 5 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for process.run");
            }
        }

        let command_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let command_ptr = self.expect_string_pointer(
            command_expr,
            "process.run expects the command to be a String",
        )?;

        let args_value = if arguments.len() >= 2 {
            let expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let env_value = if arguments.len() >= 3 {
            let expr = self.compile_expression(&arguments[2].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let cwd_value = if arguments.len() >= 4 {
            let expr = self.compile_expression(&arguments[3].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let stdin_value = if arguments.len() >= 5 {
            let expr = self.compile_expression(&arguments[4].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let template_ptr = self.ensure_struct_template("ProcessResult")?;
        let func = self.ensure_process_run_fn();
        let pointer = self
            .call_function(
                func,
                &[
                    template_ptr.into(),
                    command_ptr.as_basic_value_enum().into(),
                    args_value,
                    env_value,
                    cwd_value,
                    stdin_value,
                ],
                "tea_process_run",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_process_run returned no value"))?
            .into_pointer_value();

        Ok(ExprValue::Struct {
            pointer,
            struct_name: "ProcessResult".to_string(),
        })
    }

    fn compile_process_spawn_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.is_empty() || arguments.len() > 4 {
            bail!("process.spawn expects between 1 and 4 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for process.spawn");
            }
        }

        let command_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let command_ptr = self.expect_string_pointer(
            command_expr,
            "process.spawn expects the command to be a String",
        )?;

        let args_value = if arguments.len() >= 2 {
            let expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let env_value = if arguments.len() >= 3 {
            let expr = self.compile_expression(&arguments[2].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let cwd_value = if arguments.len() >= 4 {
            let expr = self.compile_expression(&arguments[3].expression, function, locals)?;
            self.expr_to_metadata_value(expr)?
        } else {
            self.build_nil_value()?
        };

        let func = self.ensure_process_spawn_fn();
        let value = self
            .call_function(
                func,
                &[
                    command_ptr.as_basic_value_enum().into(),
                    args_value,
                    env_value,
                    cwd_value,
                ],
                "tea_process_spawn",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_process_spawn returned no value"))?
            .into_int_value();

        Ok(ExprValue::Int(value))
    }

    fn compile_process_wait_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("process.wait expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for process.wait");
        }

        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value =
            self.expect_int_value(handle_expr, "process.wait expects the handle to be an Int")?;

        let template_ptr = self.ensure_struct_template("ProcessResult")?;
        let func = self.ensure_process_wait_fn();
        let pointer = self
            .call_function(
                func,
                &[template_ptr.into(), handle_value.into()],
                "tea_process_wait",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_process_wait returned no value"))?
            .into_pointer_value();

        Ok(ExprValue::Struct {
            pointer,
            struct_name: "ProcessResult".to_string(),
        })
    }

    fn compile_process_kill_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("process.kill expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for process.kill");
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value =
            self.expect_int_value(handle_expr, "process.kill expects the handle to be an Int")?;
        let func = self.ensure_process_kill_fn();
        let raw = self
            .call_function(func, &[handle_value.into()], "tea_process_kill")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_process_kill returned no value"))?
            .into_int_value();
        let bool_value = self.i32_to_bool(raw, "process_kill_result")?;
        Ok(ExprValue::Bool(bool_value))
    }

    fn compile_process_read_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        stdout: bool,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.is_empty() || arguments.len() > 2 {
            bail!("process.read_* expects between 1 and 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for process.read_*");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "process.read_* expects the handle to be an Int",
        )?;
        let size_value = if arguments.len() == 2 {
            let expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            self.expect_int_value(expr, "process.read_* expects bytes to be an Int")?
        } else {
            self.int_type().const_all_ones()
        };
        let func = if stdout {
            self.ensure_process_read_stdout_fn()
        } else {
            self.ensure_process_read_stderr_fn()
        };
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), size_value.into()],
                if stdout {
                    "tea_process_read_stdout"
                } else {
                    "tea_process_read_stderr"
                },
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_process_read_* returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_process_write_stdin_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("process.write_stdin expects exactly 2 arguments");
        }
        if arguments[0].name.is_some() || arguments[1].name.is_some() {
            bail!("named arguments are not supported for process.write_stdin");
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "process.write_stdin expects the handle to be an Int",
        )?;
        let data_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let data_value = self.expr_to_metadata_value(data_expr)?;
        let func = self.ensure_process_write_stdin_fn();
        self.call_function(
            func,
            &[handle_value.into(), data_value],
            "tea_process_write_stdin",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_process_close_stdin_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("process.close_stdin expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for process.close_stdin");
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "process.close_stdin expects the handle to be an Int",
        )?;
        let func = self.ensure_process_close_stdin_fn();
        self.call_function(func, &[handle_value.into()], "tea_process_close_stdin")?;
        Ok(ExprValue::Void)
    }

    fn compile_process_close_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("process.close expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for process.close");
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value =
            self.expect_int_value(handle_expr, "process.close expects the handle to be an Int")?;
        let func = self.ensure_process_close_fn();
        self.call_function(func, &[handle_value.into()], "tea_process_close")?;
        Ok(ExprValue::Void)
    }

    fn compile_json_encode_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("json.encode expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for json.encode");
        }
        let expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let tea_value = self.expr_to_tea_value(expr)?;
        let func = self.ensure_json_encode();
        let ptr = self
            .call_function(func, &[tea_value.into()], "tea_json_encode")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_json_encode returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(ptr))
    }

    fn compile_json_decode_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("json.decode expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for json.decode");
        }
        let expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let ptr = self.expect_string_pointer(expr, "json.decode expects a String argument")?;
        let func = self.ensure_json_decode();
        let value = self
            .call_function(func, &[ptr.into()], "tea_json_decode")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_json_decode returned no value"))?
            .into_struct_value();
        self.tea_value_to_expr(value, ValueType::Dict(Box::new(ValueType::Void)))
    }

    fn compile_yaml_encode_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("yaml.encode expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for yaml.encode");
        }
        let expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let tea_value = self.expr_to_tea_value(expr)?;
        let func = self.ensure_yaml_encode();
        let ptr = self
            .call_function(func, &[tea_value.into()], "tea_yaml_encode")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_yaml_encode returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(ptr))
    }

    fn compile_yaml_decode_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("yaml.decode expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for yaml.decode");
        }
        let expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let ptr = self.expect_string_pointer(expr, "yaml.decode expects a String argument")?;
        let func = self.ensure_yaml_decode();
        let value = self
            .call_function(func, &[ptr.into()], "tea_yaml_decode")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_yaml_decode returned no value"))?
            .into_struct_value();
        self.tea_value_to_expr(value, ValueType::Dict(Box::new(ValueType::Void)))
    }

    fn compile_fs_read_text_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("read_text expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for read_text");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("read_text expects the path argument to be a String"),
        };
        let func = self.ensure_fs_read_text_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_fs_read_text")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_read_text returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_fs_write_text_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("write_text expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for write_text");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("write_text expects the path argument to be a String"),
        };
        let contents_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let contents_ptr = match contents_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("write_text expects the contents argument to be a String"),
        };
        let func = self.ensure_fs_write_text_fn();
        self.call_function(
            func,
            &[path_ptr.into(), contents_ptr.into()],
            "tea_fs_write_text",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_write_text_atomic_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("write_text_atomic expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for write_text_atomic");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("write_text_atomic expects the path argument to be a String"),
        };
        let contents_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let contents_ptr = match contents_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("write_text_atomic expects the contents argument to be a String"),
        };
        let func = self.ensure_fs_write_text_atomic_fn();
        self.call_function(
            func,
            &[path_ptr.into(), contents_ptr.into()],
            "tea_fs_write_text_atomic",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_read_bytes_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("read_bytes expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for read_bytes");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("read_bytes expects the path argument to be a String"),
        };
        let func = self.ensure_fs_read_bytes_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_fs_read_bytes")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_read_bytes returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::Int),
        })
    }

    fn compile_fs_write_bytes_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("write_bytes expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for write_bytes");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("write_bytes expects the path argument to be a String"),
        };
        let data_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let list_ptr = match data_expr {
            ExprValue::List { pointer, .. } => pointer,
            _ => bail!("write_bytes expects the data argument to be a List"),
        };
        let func = self.ensure_fs_write_bytes_fn();
        self.call_function(
            func,
            &[path_ptr.into(), list_ptr.into()],
            "tea_fs_write_bytes",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_write_bytes_atomic_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("write_bytes_atomic expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for write_bytes_atomic");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("write_bytes_atomic expects the path argument to be a String"),
        };
        let list_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let list_ptr = match list_expr {
            ExprValue::List { pointer, .. } => pointer,
            _ => bail!("write_bytes_atomic expects the data argument to be a List"),
        };
        let func = self.ensure_fs_write_bytes_atomic_fn();
        self.call_function(
            func,
            &[path_ptr.into(), list_ptr.into()],
            "tea_fs_write_bytes_atomic",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_create_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if !(1..=2).contains(&arguments.len()) {
            bail!("create_dir expects 1 or 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for create_dir");
            }
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("create_dir expects the path argument to be a String"),
        };
        let recursive_flag = if arguments.len() == 2 {
            let expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            match expr {
                ExprValue::Bool(flag) => flag,
                _ => bail!("create_dir expects the optional flag to be a Bool"),
            }
        } else {
            self.bool_type().const_zero()
        };
        let recursive_c = self.bool_to_i32(recursive_flag, "fs_create_dir_recursive")?;
        let func = self.ensure_fs_create_dir_fn();
        self.call_function(
            func,
            &[path_ptr.into(), recursive_c.into()],
            "tea_fs_create_dir",
        )?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_ensure_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("ensure_dir expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for ensure_dir");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("ensure_dir expects the path argument to be a String"),
        };
        let func = self.ensure_fs_ensure_dir_fn();
        self.call_function(func, &[path_ptr.into()], "tea_fs_ensure_dir")?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_ensure_parent_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("ensure_parent expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for ensure_parent");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("ensure_parent expects the path argument to be a String"),
        };
        let func = self.ensure_fs_ensure_parent_fn();
        self.call_function(func, &[path_ptr.into()], "tea_fs_ensure_parent")?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_remove_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("remove expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for remove");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("remove expects the path argument to be a String"),
        };
        let func = self.ensure_fs_remove_fn();
        self.call_function(func, &[path_ptr.into()], "tea_fs_remove")?;
        Ok(ExprValue::Void)
    }

    fn compile_fs_exists_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("exists expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for exists");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("exists expects the path argument to be a String"),
        };
        let func = self.ensure_fs_exists_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_exists")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_exists returned no value"))?
            .into_int_value();
        let flag = self.i32_to_bool(raw, "fs_exists_result")?;
        Ok(ExprValue::Bool(flag))
    }

    fn compile_fs_is_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("is_dir expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for is_dir");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("is_dir expects the path argument to be a String"),
        };
        let func = self.ensure_fs_is_dir_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_is_dir")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_is_dir returned no value"))?
            .into_int_value();
        let flag = self.i32_to_bool(raw, "fs_is_dir_result")?;
        Ok(ExprValue::Bool(flag))
    }

    fn compile_fs_is_symlink_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("is_symlink expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for is_symlink");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("is_symlink expects the path argument to be a String"),
        };
        let func = self.ensure_fs_is_symlink_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_is_symlink")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_is_symlink returned no value"))?
            .into_int_value();
        let flag = self.i32_to_bool(raw, "fs_is_symlink_result")?;
        Ok(ExprValue::Bool(flag))
    }

    fn compile_fs_size_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("size expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for size");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("size expects the path argument to be a String"),
        };
        let func = self.ensure_fs_size_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_size")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_size returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(raw))
    }

    fn compile_fs_modified_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("modified expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for modified");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("modified expects the path argument to be a String"),
        };
        let func = self.ensure_fs_modified_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_modified")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_modified returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(raw))
    }

    fn compile_fs_permissions_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("permissions expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for permissions");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("permissions expects the path argument to be a String"),
        };
        let func = self.ensure_fs_permissions_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_permissions")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_permissions returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(raw))
    }

    fn compile_fs_is_readonly_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("is_readonly expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for is_readonly");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("is_readonly expects the path argument to be a String"),
        };
        let func = self.ensure_fs_is_readonly_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_is_readonly")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_is_readonly returned no value"))?
            .into_int_value();
        let flag = self.i32_to_bool(raw, "fs_is_readonly_result")?;
        Ok(ExprValue::Bool(flag))
    }

    fn compile_fs_list_dir_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("list_dir expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for list_dir");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("list_dir expects the path argument to be a String"),
        };
        let func = self.ensure_fs_list_dir_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_fs_list_dir")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_list_dir returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_fs_walk_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("walk expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for walk");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("walk expects the path argument to be a String"),
        };
        let func = self.ensure_fs_walk_fn();
        let pointer = self
            .call_function(func, &[path_ptr.into()], "tea_fs_walk")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_walk returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_fs_glob_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("glob expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for glob");
        }
        let pattern_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let pattern_ptr = match pattern_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("glob expects the pattern argument to be a String"),
        };
        let func = self.ensure_fs_glob_fn();
        let pointer = self
            .call_function(func, &[pattern_ptr.into()], "tea_fs_glob")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_glob returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_fs_metadata_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("metadata expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for metadata");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("metadata expects the path argument to be a String"),
        };
        let func = self.ensure_fs_metadata_fn();
        let value = self
            .call_function(func, &[path_ptr.into()], "tea_fs_metadata")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_metadata returned no value"))?
            .into_struct_value();
        self.tea_value_to_expr(value, ValueType::Dict(Box::new(ValueType::Void)))
    }

    fn compile_fs_open_read_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("open_read expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for open_read");
        }
        let path_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let path_ptr = match path_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("open_read expects the path argument to be a String"),
        };
        let func = self.ensure_fs_open_read_fn();
        let raw = self
            .call_function(func, &[path_ptr.into()], "tea_fs_open_read")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_open_read returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(raw))
    }

    fn compile_fs_read_chunk_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("read_chunk expects exactly 2 arguments");
        }
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported for read_chunk");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = handle_expr.into_int()?;
        let size_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let size_value = size_expr.into_int()?;
        let func = self.ensure_fs_read_chunk_fn();
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), size_value.into()],
                "tea_fs_read_chunk",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_fs_read_chunk returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::Int),
        })
    }

    fn compile_fs_close_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("close expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for close");
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = handle_expr.into_int()?;
        let func = self.ensure_fs_close_fn();
        self.call_function(func, &[handle_value.into()], "tea_fs_close")?;
        Ok(ExprValue::Void)
    }

    fn call_closure(
        &mut self,
        closure_ptr: PointerValue<'ctx>,
        param_types: &[ValueType],
        return_type: &ValueType,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if param_types.len() != call.arguments.len() {
            bail!(
                "closure expects {} arguments, found {}",
                param_types.len(),
                call.arguments.len()
            );
        }

        let mut arg_values: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(call.arguments.len());
        for (index, (expected_type, argument)) in
            param_types.iter().zip(call.arguments.iter()).enumerate()
        {
            if argument.name.is_some() {
                bail!("named arguments are not supported by the LLVM backend yet");
            }
            let value = self.compile_expression(&argument.expression, function, locals)?;
            let converted = self
                .convert_expr_to_type(value, expected_type)
                .map_err(|error| {
                    anyhow!(
                        "argument {} to closure has mismatched type: {}",
                        index + 1,
                        error
                    )
                })?;
            let basic = converted
                .into_basic_value()
                .ok_or_else(|| anyhow!("argument must produce a value"))?;
            arg_values.push(basic);
        }

        let fn_ptr_ptr = map_builder_error(self.builder.build_struct_gep(
            self.tea_closure,
            closure_ptr,
            0,
            "closure_fn_ptr",
        ))?;
        let raw_fn_ptr = map_builder_error(self.builder.build_load(
            self.ptr_type,
            fn_ptr_ptr,
            "closure_fn",
        ))?
        .into_pointer_value();

        let mut llvm_params: Vec<BasicMetadataTypeEnum<'ctx>> =
            Vec::with_capacity(param_types.len() + 1);
        llvm_params.push(self.closure_ptr_type().into());
        for ty in param_types {
            llvm_params.push(self.basic_type(ty)?.into());
        }

        let fn_type = match return_type {
            ValueType::Void => self.context.void_type().fn_type(&llvm_params, false),
            ValueType::Int => self.int_type().fn_type(&llvm_params, false),
            ValueType::Float => self.float_type().fn_type(&llvm_params, false),
            ValueType::Bool => self.bool_type().fn_type(&llvm_params, false),
            ValueType::String => self.string_ptr_type().fn_type(&llvm_params, false),
            ValueType::List(_) => self.list_ptr_type().fn_type(&llvm_params, false),
            ValueType::Dict(_) => self.dict_ptr_type().fn_type(&llvm_params, false),
            ValueType::Struct(_) => self.struct_ptr_type().fn_type(&llvm_params, false),
            ValueType::Error { .. } => self.error_ptr_type().fn_type(&llvm_params, false),
            ValueType::Function(_, _) => self.closure_ptr_type().fn_type(&llvm_params, false),
            ValueType::Optional(_) => self.value_type().fn_type(&llvm_params, false),
        };

        let typed_fn_ptr = map_builder_error(self.builder.build_bit_cast(
            raw_fn_ptr,
            self.ptr_type,
            "closure_callee",
        ))?
        .into_pointer_value();

        let mut args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::with_capacity(arg_values.len() + 1);
        args.push(closure_ptr.into());
        for value in arg_values {
            args.push(BasicMetadataValueEnum::from(value));
        }

        let call_site = map_builder_error(self.builder.build_indirect_call(
            fn_type,
            typed_fn_ptr,
            &args,
            "closure_call",
        ))?;

        if matches!(return_type, ValueType::Void) {
            self.handle_possible_error(function)?;
            return Ok(ExprValue::Void);
        }

        let result = call_site
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("closure returned no value"))?;

        let expr = match return_type {
            ValueType::Int => ExprValue::Int(result.into_int_value()),
            ValueType::Float => ExprValue::Float(result.into_float_value()),
            ValueType::Bool => ExprValue::Bool(result.into_int_value()),
            ValueType::String => ExprValue::String(result.into_pointer_value()),
            ValueType::List(inner) => ExprValue::List {
                pointer: result.into_pointer_value(),
                element_type: inner.clone(),
            },
            ValueType::Dict(inner) => ExprValue::Dict {
                pointer: result.into_pointer_value(),
                value_type: inner.clone(),
            },
            ValueType::Struct(struct_name) => ExprValue::Struct {
                pointer: result.into_pointer_value(),
                struct_name: struct_name.clone(),
            },
            ValueType::Error {
                error_name,
                variant_name,
            } => ExprValue::Error {
                pointer: result.into_pointer_value(),
                error_name: error_name.clone(),
                variant_name: variant_name.clone(),
            },
            ValueType::Function(params, ret) => ExprValue::Closure {
                pointer: result.into_pointer_value(),
                param_types: params.clone(),
                return_type: ret.clone(),
            },
            ValueType::Optional(inner) => ExprValue::Optional {
                value: result.into_struct_value(),
                inner: inner.clone(),
            },
            ValueType::Void => unreachable!(),
        };
        self.handle_possible_error(function)?;

        Ok(expr)
    }

    fn compile_struct_constructor(
        &mut self,
        name: &str,
        span: SourceSpan,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let (field_names, field_types_default) = {
            let info = self
                .structs
                .get(name)
                .ok_or_else(|| anyhow!(format!("unknown struct '{name}'")))?;
            (info.field_names.clone(), info.field_types.clone())
        };
        let mut field_types = field_types_default.clone();
        let mut variant_name = name.to_string();
        if let Some((_, instance)) = self.struct_call_metadata_tc.get(&span) {
            let instance = instance.clone();
            let mut converted = Vec::with_capacity(instance.field_types.len());
            for ty in &instance.field_types {
                converted.push(self.resolve_type_with_bindings_to_value(ty)?);
            }
            field_types = converted;
            let mut resolved_args = Vec::with_capacity(instance.type_arguments.len());
            for arg in &instance.type_arguments {
                resolved_args.push(self.resolve_type_with_bindings(arg)?);
            }
            let struct_type = StructType {
                name: name.to_string(),
                type_arguments: resolved_args,
            };
            variant_name = format_struct_type_name(&struct_type);
            self.struct_field_variants
                .entry(variant_name.clone())
                .or_insert_with(|| field_types.clone());
            self.struct_variant_bases
                .entry(variant_name.clone())
                .or_insert_with(|| name.to_string());
        } else {
            if field_types.is_empty() {
                bail!(format!("missing struct field type metadata for '{}'", name));
            }
            self.struct_field_variants
                .entry(name.to_string())
                .or_insert_with(|| field_types.clone());
            self.struct_variant_bases
                .entry(name.to_string())
                .or_insert_with(|| name.to_string());
        }

        let template_ptr = self.ensure_struct_template(name)?;
        let alloc_fn = self.ensure_alloc_struct();
        let call = self.call_function(alloc_fn, &[template_ptr.into()], "struct_alloc")?;
        let struct_ptr = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected struct pointer"))?
            .into_pointer_value();
        let set_fn = self.ensure_struct_set();

        let field_count = field_names.len();
        let has_named = arguments.iter().any(|arg| arg.name.is_some());
        if has_named && arguments.iter().any(|arg| arg.name.is_none()) {
            bail!("cannot mix named and positional arguments in struct constructor");
        }

        if !has_named && arguments.len() != field_count {
            bail!(
                "struct '{}' expects {} arguments, found {}",
                name,
                field_count,
                arguments.len()
            );
        }

        if has_named {
            let mut lookup = HashMap::with_capacity(field_count);
            for (index, field_name) in field_names.iter().enumerate() {
                lookup.insert(field_name.clone(), index);
            }
            let mut seen = vec![false; field_count];
            for argument in arguments {
                let field_name = argument
                    .name
                    .as_ref()
                    .expect("named argument expected for struct");
                let index = lookup.get(field_name.as_str()).copied().ok_or_else(|| {
                    anyhow!(format!(
                        "struct '{}' has no field named '{}'",
                        name, field_name
                    ))
                })?;
                if seen[index] {
                    bail!(format!(
                        "field '{}' provided multiple times to struct '{}'",
                        field_name, name
                    ));
                }
                let value = self.compile_expression(&argument.expression, function, locals)?;
                let expected = &field_types[index];
                let converted = self
                    .convert_expr_to_type(value, expected)
                    .map_err(|error| {
                        anyhow!(
                            "field '{}' in struct '{}' expects {:?}: {}",
                            field_name,
                            name,
                            expected,
                            error
                        )
                    })?;
                let tea_value = BasicMetadataValueEnum::from(self.expr_to_tea_value(converted)?);
                self.call_function(
                    set_fn,
                    &[
                        struct_ptr.into(),
                        self.int_type().const_int(index as u64, false).into(),
                        tea_value,
                    ],
                    "struct_set",
                )?;
                seen[index] = true;
            }
            if seen.iter().any(|assigned| !*assigned) {
                let missing: Vec<&str> = field_names
                    .iter()
                    .enumerate()
                    .filter_map(|(index, field)| (!seen[index]).then(|| field.as_str()))
                    .collect();
                if !missing.is_empty() {
                    bail!(format!(
                        "missing fields for struct '{}': {}",
                        name,
                        missing.join(", ")
                    ));
                }
            }
        } else {
            for (index, argument) in arguments.iter().enumerate() {
                if argument.name.is_some() {
                    bail!("cannot mix named arguments in positional struct constructor");
                }
                let value = self.compile_expression(&argument.expression, function, locals)?;
                let expected = &field_types[index];
                let converted = self
                    .convert_expr_to_type(value, expected)
                    .map_err(|error| {
                        anyhow!(
                            "field '{}' in struct '{}' expects {:?}: {}",
                            field_names[index],
                            name,
                            expected,
                            error
                        )
                    })?;
                let tea_value = BasicMetadataValueEnum::from(self.expr_to_tea_value(converted)?);
                self.call_function(
                    set_fn,
                    &[
                        struct_ptr.into(),
                        self.int_type().const_int(index as u64, false).into(),
                        tea_value,
                    ],
                    "struct_set",
                )?;
            }
        }

        Ok(ExprValue::Struct {
            pointer: struct_ptr,
            struct_name: variant_name,
        })
    }

    fn try_compile_error_constructor(
        &mut self,
        error_name: &str,
        variant: Option<&str>,
        call: &CallExpression,
        _span: SourceSpan,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<Option<ExprValue<'ctx>>> {
        let definition = match self.error_definitions_tc.get(error_name) {
            Some(def) => def,
            None => return Ok(None),
        };

        let (variant_name, variant_def) = match variant {
            Some(name) => match definition.variants.get(name) {
                Some(def) => (name.to_string(), def),
                None => return Ok(None),
            },
            None => {
                if definition.variants.len() == 1 {
                    let (name, def) = definition
                        .variants
                        .iter()
                        .next()
                        .expect("error with single variant");
                    (name.clone(), def)
                } else {
                    return Ok(None);
                }
            }
        };

        if call.arguments.len() != variant_def.fields.len() {
            bail!(format!(
                "error variant '{}.{}' expects {} argument(s) but {} provided",
                error_name,
                variant_name,
                variant_def.fields.len(),
                call.arguments.len()
            ));
        }

        self.ensure_error_variant_metadata(error_name, &variant_name)?;
        let field_types = self
            .errors
            .get(error_name)
            .and_then(|variants| variants.get(&variant_name))
            .map(|entry| entry.field_types.clone())
            .ok_or_else(|| {
                anyhow!(
                    "missing field metadata for error '{}.{}'",
                    error_name,
                    variant_name
                )
            })?;
        let template_ptr = self.ensure_error_template(error_name, &variant_name)?;

        let alloc_fn = self.ensure_error_alloc();
        let call_site = self.call_function(alloc_fn, &[template_ptr.into()], "error_alloc")?;
        let error_ptr = call_site
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected error pointer"))?
            .into_pointer_value();

        let set_fn = self.ensure_error_set();
        for (index, argument) in call.arguments.iter().enumerate() {
            if argument.name.is_some() {
                bail!("named arguments are not supported for error constructors");
            }
            let value = self.compile_expression(&argument.expression, function, locals)?;
            let expected = field_types.get(index).ok_or_else(|| {
                anyhow!(
                    "missing field metadata for error '{}.{}'",
                    error_name,
                    variant_name
                )
            })?;
            let converted = self
                .convert_expr_to_type(value, expected)
                .map_err(|error| {
                    anyhow!(
                        "argument {} for error '{}.{}' has mismatched type: {}",
                        index + 1,
                        error_name,
                        variant_name,
                        error
                    )
                })?;
            let tea_value = self.expr_to_tea_value(converted)?;
            self.call_function(
                set_fn,
                &[
                    error_ptr.into(),
                    self.int_type().const_int(index as u64, false).into(),
                    tea_value.into(),
                ],
                "error_set",
            )?;
        }

        Ok(Some(ExprValue::Error {
            pointer: error_ptr,
            error_name: error_name.to_string(),
            variant_name: Some(variant_name),
        }))
    }

    fn compile_print_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported by the LLVM backend yet");
            }
            let value = self.compile_expression(&argument.expression, function, locals)?;
            match value {
                ExprValue::Int(v) => {
                    let func = self.ensure_print_int();
                    self.call_function(func, &[v.into()], "print_int")?;
                }
                ExprValue::Float(v) => {
                    let func = self.ensure_print_float();
                    self.call_function(func, &[v.into()], "print_float")?;
                }
                ExprValue::Bool(v) => {
                    let cast = self.bool_to_i32(v, "print_bool")?;
                    let func = self.ensure_print_bool();
                    self.call_function(func, &[cast.into()], "print_bool")?;
                }
                ExprValue::String(ptr) => {
                    let func = self.ensure_print_string();
                    self.call_function(func, &[ptr.into()], "print_string")?;
                }
                ExprValue::List { pointer, .. } => {
                    let func = self.ensure_print_list();
                    self.call_function(func, &[pointer.into()], "print_list")?;
                }
                ExprValue::Dict { pointer, .. } => {
                    let func = self.ensure_print_dict();
                    self.call_function(func, &[pointer.into()], "print_dict")?;
                }
                ExprValue::Struct { pointer, .. } => {
                    let func = self.ensure_print_struct();
                    self.call_function(func, &[pointer.into()], "print_struct")?;
                }
                ExprValue::Error { pointer, .. } => {
                    let func = self.ensure_print_error();
                    self.call_function(func, &[pointer.into()], "print_error")?;
                }
                ExprValue::Closure { pointer, .. } => {
                    let func = self.ensure_print_closure();
                    self.call_function(func, &[pointer.into()], "print_closure")?;
                }
                ExprValue::Optional { value, .. } => {
                    let to_string = self.ensure_util_to_string_fn();
                    let string_ptr = self
                        .call_function(to_string, &[value.into()], "optional_to_string")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("tea_util_to_string returned no value"))?
                        .into_pointer_value();
                    let func = self.ensure_print_string();
                    self.call_function(func, &[string_ptr.into()], "print_optional")?;
                }
                ExprValue::Void => {
                    let nil_string = self.compile_string_literal("nil")?;
                    if let ExprValue::String(ptr) = nil_string {
                        let func = self.ensure_print_string();
                        self.call_function(func, &[ptr.into()], "print_nil")?;
                    } else {
                        bail!("expected string literal for nil printing");
                    }
                }
            }
        }
        Ok(ExprValue::Void)
    }

    fn build_numeric_add(
        &mut self,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        match (left, right) {
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => Ok(ExprValue::Int(map_builder_error(
                self.builder.build_int_add(lhs, rhs, "addtmp"),
            )?)),
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => Ok(ExprValue::Float(
                map_builder_error(self.builder.build_float_add(lhs, rhs, "faddtmp"))?,
            )),
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_add_l")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_add(lhs, rhs, "faddtmp"),
                )?))
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_add_r")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_add(lhs, rhs, "faddtmp"),
                )?))
            }
            _ => bail!("add expects numeric operands"),
        }
    }

    fn build_numeric_sub(
        &mut self,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        match (left, right) {
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => Ok(ExprValue::Int(map_builder_error(
                self.builder.build_int_sub(lhs, rhs, "subtmp"),
            )?)),
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => Ok(ExprValue::Float(
                map_builder_error(self.builder.build_float_sub(lhs, rhs, "fsubtmp"))?,
            )),
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_sub_l")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_sub(lhs, rhs, "fsubtmp"),
                )?))
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_sub_r")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_sub(lhs, rhs, "fsubtmp"),
                )?))
            }
            _ => bail!("sub expects numeric operands"),
        }
    }

    fn build_numeric_mul(
        &mut self,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        match (left, right) {
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => Ok(ExprValue::Int(map_builder_error(
                self.builder.build_int_mul(lhs, rhs, "multmp"),
            )?)),
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => Ok(ExprValue::Float(
                map_builder_error(self.builder.build_float_mul(lhs, rhs, "fmultmp"))?,
            )),
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_mul_l")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_mul(lhs, rhs, "fmultmp"),
                )?))
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_mul_r")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_mul(lhs, rhs, "fmultmp"),
                )?))
            }
            _ => bail!("mul expects numeric operands"),
        }
    }

    fn build_numeric_div(
        &mut self,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        match (left, right) {
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => Ok(ExprValue::Int(map_builder_error(
                self.builder.build_int_signed_div(lhs, rhs, "divtmp"),
            )?)),
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => Ok(ExprValue::Float(
                map_builder_error(self.builder.build_float_div(lhs, rhs, "fdivtmp"))?,
            )),
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_div_l")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_div(lhs, rhs, "fdivtmp"),
                )?))
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_div_r")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_div(lhs, rhs, "fdivtmp"),
                )?))
            }
            _ => bail!("div expects numeric operands"),
        }
    }

    fn build_numeric_mod(
        &mut self,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        match (left, right) {
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => Ok(ExprValue::Int(map_builder_error(
                self.builder.build_int_signed_rem(lhs, rhs, "modtmp"),
            )?)),
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => Ok(ExprValue::Float(
                map_builder_error(self.builder.build_float_rem(lhs, rhs, "fmodtmp"))?,
            )),
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_mod_l")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_rem(lhs, rhs, "fmodtmp"),
                )?))
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_mod_r")?;
                Ok(ExprValue::Float(map_builder_error(
                    self.builder.build_float_rem(lhs, rhs, "fmodtmp"),
                )?))
            }
            _ => bail!("mod expects numeric operands"),
        }
    }

    fn build_equality(
        &mut self,
        function: FunctionValue<'ctx>,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
        is_equal: bool,
    ) -> Result<ExprValue<'ctx>> {
        let (int_pred, float_pred) = if is_equal {
            (IntPredicate::EQ, FloatPredicate::OEQ)
        } else {
            (IntPredicate::NE, FloatPredicate::ONE)
        };

        let result = match (left, right) {
            (ExprValue::Optional { value, .. }, ExprValue::Void) => {
                let is_nil_fn = self.ensure_util_is_nil_fn();
                let call = self.call_function(is_nil_fn, &[value.into()], "optional_eq_nil")?;
                let raw = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_util_is_nil"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "optional_eq_nil_bool")?;
                if is_equal {
                    bool_val
                } else {
                    map_builder_error(self.builder.build_not(bool_val, "optional_ne_nil"))?
                }
            }
            (ExprValue::Void, optional @ ExprValue::Optional { .. }) => {
                return self.build_equality(function, optional, ExprValue::Void, is_equal);
            }
            (
                ExprValue::Optional {
                    value: left_value,
                    inner: left_inner,
                },
                ExprValue::Optional {
                    value: right_value,
                    inner: right_inner,
                },
            ) => {
                if left_inner != right_inner {
                    bail!("cannot compare optionals with different inner types");
                }

                let is_nil_fn = self.ensure_util_is_nil_fn();
                let left_nil_call =
                    self.call_function(is_nil_fn, &[left_value.into()], "opt_eq_left_nil")?;
                let left_nil_raw = left_nil_call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_util_is_nil"))?
                    .into_int_value();
                let left_nil = self.i32_to_bool(left_nil_raw, "opt_left_nil_bool")?;

                let right_nil_call =
                    self.call_function(is_nil_fn, &[right_value.into()], "opt_eq_right_nil")?;
                let right_nil_raw = right_nil_call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_util_is_nil"))?
                    .into_int_value();
                let right_nil = self.i32_to_bool(right_nil_raw, "opt_right_nil_bool")?;

                let either_nil = map_builder_error(self.builder.build_or(
                    left_nil,
                    right_nil,
                    "opt_either_nil",
                ))?;
                let nil_block = self.context.append_basic_block(function, "opt_eq_nil");
                let non_nil_block = self.context.append_basic_block(function, "opt_eq_non_nil");
                let merge_block = self.context.append_basic_block(function, "opt_eq_merge");

                map_builder_error(self.builder.build_conditional_branch(
                    either_nil,
                    nil_block,
                    non_nil_block,
                ))?;

                self.builder.position_at_end(nil_block);
                let both_nil =
                    map_builder_error(self.builder.build_and(left_nil, right_nil, "opt_both_nil"))?;
                let nil_result = if is_equal {
                    both_nil
                } else {
                    map_builder_error(self.builder.build_not(both_nil, "opt_nil_ne"))?
                };
                map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
                let nil_block_end = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| anyhow!("missing optional nil block"))?;

                self.builder.position_at_end(non_nil_block);
                let inner_type = (*left_inner).clone();
                let right_inner_type = inner_type.clone();
                let left_inner_expr = self.tea_value_to_expr(left_value, inner_type.clone())?;
                let right_inner_expr = self.tea_value_to_expr(right_value, right_inner_type)?;
                let inner_cmp =
                    self.build_equality(function, left_inner_expr, right_inner_expr, true)?;
                let inner_bool = match inner_cmp {
                    ExprValue::Bool(value) => value,
                    _ => bail!("inner optional comparison did not produce a bool"),
                };
                let non_nil_result = if is_equal {
                    inner_bool
                } else {
                    map_builder_error(self.builder.build_not(inner_bool, "opt_non_nil_ne"))?
                };
                map_builder_error(self.builder.build_unconditional_branch(merge_block))?;
                let non_nil_block_end = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| anyhow!("missing optional non-nil block"))?;

                self.builder.position_at_end(merge_block);
                let phi =
                    map_builder_error(self.builder.build_phi(self.bool_type(), "opt_eq_phi"))?;
                let nil_basic = nil_result.as_basic_value_enum();
                let non_nil_basic = non_nil_result.as_basic_value_enum();
                phi.add_incoming(&[
                    (&nil_basic, nil_block_end),
                    (&non_nil_basic, non_nil_block_end),
                ]);
                phi.as_basic_value().into_int_value()
            }
            (ExprValue::Optional { value, inner }, other) => {
                let expected = ValueType::Optional(inner.clone());
                let converted = self.convert_expr_to_type(other, &expected)?;
                return self.build_equality(
                    function,
                    ExprValue::Optional { value, inner },
                    converted,
                    is_equal,
                );
            }
            (other, ExprValue::Optional { value, inner }) => {
                let expected = ValueType::Optional(inner.clone());
                let converted = self.convert_expr_to_type(other, &expected)?;
                return self.build_equality(
                    function,
                    converted,
                    ExprValue::Optional { value, inner },
                    is_equal,
                );
            }
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => {
                map_builder_error(self.builder.build_int_compare(int_pred, lhs, rhs, "cmptmp"))?
            }
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => map_builder_error(
                self.builder
                    .build_float_compare(float_pred, lhs, rhs, "fcmptmp"),
            )?,
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_eq_l")?;
                map_builder_error(
                    self.builder
                        .build_float_compare(float_pred, lhs, rhs, "fcmptmp"),
                )?
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_eq_r")?;
                map_builder_error(
                    self.builder
                        .build_float_compare(float_pred, lhs, rhs, "fcmptmp"),
                )?
            }
            (ExprValue::Bool(lhs), ExprValue::Bool(rhs)) => {
                map_builder_error(self.builder.build_int_compare(int_pred, lhs, rhs, "beqtmp"))?
            }
            (ExprValue::String(lhs), ExprValue::String(rhs)) => {
                let func = self.ensure_string_equal();
                let call = self.call_function(func, &[lhs.into(), rhs.into()], "str_eq")?;
                let raw = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_string_equal"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "str_eq_bool")?;
                if is_equal {
                    bool_val
                } else {
                    map_builder_error(self.builder.build_not(bool_val, "str_neq"))?
                }
            }
            (ExprValue::List { pointer: lhs, .. }, ExprValue::List { pointer: rhs, .. }) => {
                let func = self.ensure_list_equal();
                let call = self.call_function(func, &[lhs.into(), rhs.into()], "list_eq")?;
                let raw = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_list_equal"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "list_eq_bool")?;
                if is_equal {
                    bool_val
                } else {
                    map_builder_error(self.builder.build_not(bool_val, "list_neq"))?
                }
            }
            (
                ExprValue::Struct {
                    pointer: lhs,
                    struct_name: left_name,
                },
                ExprValue::Struct {
                    pointer: rhs,
                    struct_name: right_name,
                },
            ) => {
                if left_name != right_name {
                    bail!(
                        "cannot compare structs '{}' and '{}'",
                        left_name,
                        right_name
                    );
                }
                let func = self.ensure_struct_equal();
                let call = self.call_function(func, &[lhs.into(), rhs.into()], "struct_eq")?;
                let raw = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_struct_equal"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "struct_eq_bool")?;
                if is_equal {
                    bool_val
                } else {
                    map_builder_error(self.builder.build_not(bool_val, "struct_neq"))?
                }
            }
            (ExprValue::Closure { pointer: lhs, .. }, ExprValue::Closure { pointer: rhs, .. }) => {
                let func = self.ensure_closure_equal();
                let call = self.call_function(func, &[lhs.into(), rhs.into()], "closure_eq")?;
                let raw = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_closure_equal"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "closure_eq_bool")?;
                if is_equal {
                    bool_val
                } else {
                    map_builder_error(self.builder.build_not(bool_val, "closure_neq"))?
                }
            }
            (ExprValue::Dict { pointer: lhs, .. }, ExprValue::Dict { pointer: rhs, .. }) => {
                let func = self.ensure_dict_equal();
                let call = self.call_function(func, &[lhs.into(), rhs.into()], "dict_eq")?;
                let raw = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected bool from tea_dict_equal"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "dict_eq_bool")?;
                if is_equal {
                    bool_val
                } else {
                    map_builder_error(self.builder.build_not(bool_val, "dict_neq"))?
                }
            }
            (ExprValue::String(ptr), ExprValue::Void)
            | (ExprValue::Void, ExprValue::String(ptr)) => {
                let is_null = map_builder_error(self.builder.build_is_null(ptr, "str_is_null"))?;
                if is_equal {
                    is_null
                } else {
                    map_builder_error(self.builder.build_not(is_null, "str_not_null"))?
                }
            }
            (ExprValue::Void, ExprValue::Void) => self
                .bool_type()
                .const_int(if is_equal { 1 } else { 0 }, false),
            _ => bail!("unsupported equality comparison"),
        };

        Ok(ExprValue::Bool(result))
    }

    fn build_numeric_compare(
        &mut self,
        left: ExprValue<'ctx>,
        right: ExprValue<'ctx>,
        int_predicate: IntPredicate,
        float_predicate: FloatPredicate,
    ) -> Result<ExprValue<'ctx>> {
        let result = match (left, right) {
            (ExprValue::Int(lhs), ExprValue::Int(rhs)) => map_builder_error(
                self.builder
                    .build_int_compare(int_predicate, lhs, rhs, "cmptmp"),
            )?,
            (ExprValue::Float(lhs), ExprValue::Float(rhs)) => map_builder_error(
                self.builder
                    .build_float_compare(float_predicate, lhs, rhs, "fcmptmp"),
            )?,
            (ExprValue::Int(lhs), ExprValue::Float(rhs)) => {
                let lhs = self.cast_int_to_float(lhs, "sitofp_cmp_l")?;
                map_builder_error(self.builder.build_float_compare(
                    float_predicate,
                    lhs,
                    rhs,
                    "fcmptmp",
                ))?
            }
            (ExprValue::Float(lhs), ExprValue::Int(rhs)) => {
                let rhs = self.cast_int_to_float(rhs, "sitofp_cmp_r")?;
                map_builder_error(self.builder.build_float_compare(
                    float_predicate,
                    lhs,
                    rhs,
                    "fcmptmp",
                ))?
            }
            _ => bail!("comparison expects numeric operands"),
        };
        Ok(ExprValue::Bool(result))
    }

    fn build_logical_and(
        &mut self,
        left_expr: &Expression,
        right_expr: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let left = self
            .compile_expression(left_expr, function, locals)?
            .into_bool()?;
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing block"))?;

        let rhs_block = self.context.append_basic_block(function, "and_rhs");
        let merge_block = self.context.append_basic_block(function, "and_merge");

        map_builder_error(
            self.builder
                .build_conditional_branch(left, rhs_block, merge_block),
        )?;

        self.builder.position_at_end(rhs_block);
        let right = self
            .compile_expression(right_expr, function, locals)?
            .into_bool()?;
        let rhs_end = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing rhs block"))?;
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;

        self.builder.position_at_end(merge_block);
        let phi = map_builder_error(self.builder.build_phi(self.bool_type(), "andtmp"))?;
        let false_val = self.bool_type().const_int(0, false);
        let false_val = false_val.as_basic_value_enum();
        let right_val = right.as_basic_value_enum();
        phi.add_incoming(&[(&false_val, current_block), (&right_val, rhs_end)]);
        Ok(ExprValue::Bool(phi.as_basic_value().into_int_value()))
    }

    fn build_logical_or(
        &mut self,
        left_expr: &Expression,
        right_expr: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let left = self
            .compile_expression(left_expr, function, locals)?
            .into_bool()?;
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing block"))?;

        let rhs_block = self.context.append_basic_block(function, "or_rhs");
        let merge_block = self.context.append_basic_block(function, "or_merge");

        map_builder_error(
            self.builder
                .build_conditional_branch(left, merge_block, rhs_block),
        )?;

        self.builder.position_at_end(rhs_block);
        let right = self
            .compile_expression(right_expr, function, locals)?
            .into_bool()?;
        let rhs_end = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing rhs block"))?;
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;

        self.builder.position_at_end(merge_block);
        let phi = map_builder_error(self.builder.build_phi(self.bool_type(), "ortmp"))?;
        let true_val = self.bool_type().const_int(1, false);
        let true_val = true_val.as_basic_value_enum();
        let right_val = right.as_basic_value_enum();
        phi.add_incoming(&[(&true_val, current_block), (&right_val, rhs_end)]);
        Ok(ExprValue::Bool(phi.as_basic_value().into_int_value()))
    }

    fn expr_to_tea_value(&mut self, value: ExprValue<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        match value {
            ExprValue::Int(v) => {
                let func = self.ensure_value_from_int();
                let call = self.call_function(func, &[v.into()], "val_int")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Float(v) => {
                let func = self.ensure_value_from_float();
                let call = self.call_function(func, &[v.into()], "val_float")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Bool(v) => {
                let cast = self.bool_to_i32(v, "bool_i32")?;
                let func = self.ensure_value_from_bool();
                let call = self.call_function(func, &[cast.into()], "val_bool")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::String(ptr) => {
                let func = self.ensure_value_from_string();
                let call = self.call_function(func, &[ptr.into()], "val_str")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::List { pointer, .. } => {
                let func = self.ensure_value_from_list();
                let call = self.call_function(func, &[pointer.into()], "val_list")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Dict { pointer, .. } => {
                let func = self.ensure_value_from_dict();
                let call = self.call_function(func, &[pointer.into()], "val_dict")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Struct { pointer, .. } => {
                let func = self.ensure_value_from_struct();
                let call = self.call_function(func, &[pointer.into()], "val_struct")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Error { pointer, .. } => {
                let func = self.ensure_value_from_error();
                let call = self.call_function(func, &[pointer.into()], "val_error")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Closure { pointer, .. } => {
                let func = self.ensure_value_from_closure();
                let call = self.call_function(func, &[pointer.into()], "val_closure")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Void => {
                let func = self.ensure_value_nil();
                let call = self.call_function(func, &[], "val_nil")?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected TeaValue"))?)
            }
            ExprValue::Optional { value, .. } => Ok(value.into()),
        }
    }

    fn optional_nil(&mut self, inner: &ValueType) -> Result<ExprValue<'ctx>> {
        let func = self.ensure_value_nil();
        let call = self.call_function(func, &[], "optional_nil")?;
        let value = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected TeaValue for optional nil"))?
            .into_struct_value();
        Ok(ExprValue::Optional {
            value,
            inner: Box::new(inner.clone()),
        })
    }

    fn convert_expr_to_type(
        &mut self,
        value: ExprValue<'ctx>,
        target: &ValueType,
    ) -> Result<ExprValue<'ctx>> {
        match target {
            ValueType::Optional(inner) => match value {
                ExprValue::Optional {
                    inner: ref current, ..
                } => {
                    if **current == **inner {
                        Ok(value)
                    } else {
                        bail!(
                            "optional conversion mismatch: expected {:?}, found {:?}",
                            inner,
                            current
                        )
                    }
                }
                ExprValue::Void => self.optional_nil(inner),
                other => {
                    let converted = self.convert_expr_to_type(other, inner)?;
                    let tea_value = self.expr_to_tea_value(converted)?;
                    let struct_value = tea_value.into_struct_value();
                    Ok(ExprValue::Optional {
                        value: struct_value,
                        inner: inner.clone(),
                    })
                }
            },
            ValueType::Error {
                error_name: target_name,
                variant_name: target_variant,
            } => match value {
                ExprValue::Error {
                    pointer,
                    error_name,
                    variant_name,
                } => {
                    if error_name != *target_name {
                        bail!(
                            "error type mismatch: expected error '{}', found '{}'",
                            target_name,
                            error_name
                        );
                    }
                    if let Some(target_variant) = target_variant {
                        match &variant_name {
                            Some(actual) if actual == target_variant => {}
                            Some(actual) => {
                                bail!(
                                    "error variant mismatch: expected '{}.{}', found '{}.{}'",
                                    target_name,
                                    target_variant,
                                    error_name,
                                    actual
                                );
                            }
                            None => {
                                bail!(
                                    "error variant mismatch: expected '{}.{}', found '{}'",
                                    target_name,
                                    target_variant,
                                    error_name
                                );
                            }
                        }
                    }
                    Ok(ExprValue::Error {
                        pointer,
                        error_name,
                        variant_name,
                    })
                }
                other => bail!(
                    "expected error value '{}', found {:?}",
                    target_name,
                    other.ty()
                ),
            },
            _ => {
                if value.ty() == *target {
                    Ok(value)
                } else {
                    bail!(
                        "type mismatch: expected {:?}, found {:?}",
                        target,
                        value.ty()
                    );
                }
            }
        }
    }

    fn store_expr_in_pointer(
        &mut self,
        pointer: PointerValue<'ctx>,
        ty: &ValueType,
        value: ExprValue<'ctx>,
        name: &str,
    ) -> Result<()> {
        let converted = self.convert_expr_to_type(value, ty)?;
        if let Some(basic) = converted.into_basic_value() {
            map_builder_error(self.builder.build_store(pointer, basic))?;
        } else if !matches!(ty, ValueType::Void) {
            bail!(
                "unable to store expression into '{}': expected {:?} to materialise a value",
                name,
                ty
            );
        }
        Ok(())
    }

    fn emit_return_value(&mut self, value: ExprValue<'ctx>, ty: &ValueType) -> Result<()> {
        self.clear_error_state()?;
        let value_ty = value.ty();
        match (ty, value) {
            (ValueType::Int, ExprValue::Int(v)) => {
                map_builder_error(self.builder.build_return(Some(&v)))?;
                Ok(())
            }
            (ValueType::Float, ExprValue::Float(v)) => {
                map_builder_error(self.builder.build_return(Some(&v)))?;
                Ok(())
            }
            (ValueType::Bool, ExprValue::Bool(v)) => {
                map_builder_error(self.builder.build_return(Some(&v)))?;
                Ok(())
            }
            (ValueType::String, ExprValue::String(ptr)) => {
                map_builder_error(self.builder.build_return(Some(&ptr)))?;
                Ok(())
            }
            (ValueType::List(_), ExprValue::List { pointer, .. }) => {
                map_builder_error(self.builder.build_return(Some(&pointer)))?;
                Ok(())
            }
            (ValueType::Dict(_), ExprValue::Dict { pointer, .. }) => {
                map_builder_error(self.builder.build_return(Some(&pointer)))?;
                Ok(())
            }
            (ValueType::Struct(_), ExprValue::Struct { pointer, .. }) => {
                map_builder_error(self.builder.build_return(Some(&pointer)))?;
                Ok(())
            }
            (
                ValueType::Error {
                    error_name: expected_name,
                    variant_name: expected_variant,
                },
                ExprValue::Error {
                    pointer,
                    error_name,
                    variant_name,
                },
            ) => {
                if error_name != *expected_name {
                    bail!(
                        "return type mismatch: expected error '{}', found '{}'",
                        expected_name,
                        error_name
                    );
                }
                if let Some(expected_variant) = expected_variant {
                    match &variant_name {
                        Some(actual) if actual == expected_variant => {}
                        Some(actual) => {
                            bail!(
                                "return type mismatch: expected '{}.{}', found '{}.{}'",
                                expected_name,
                                expected_variant,
                                error_name,
                                actual
                            );
                        }
                        None => {
                            bail!(
                                "return type mismatch: expected '{}.{}', found '{}' without variant",
                                expected_name,
                                expected_variant,
                                error_name
                            );
                        }
                    }
                }
                map_builder_error(self.builder.build_return(Some(&pointer)))?;
                Ok(())
            }
            (ValueType::Function(_, _), ExprValue::Closure { pointer, .. }) => {
                map_builder_error(self.builder.build_return(Some(&pointer)))?;
                Ok(())
            }
            (ValueType::Optional(expected_inner), ExprValue::Optional { value, inner }) => {
                if *expected_inner != inner {
                    bail!(
                        "return type mismatch: expected Optional {:?}, found Optional {:?}",
                        expected_inner,
                        inner
                    );
                }
                map_builder_error(self.builder.build_return(Some(&value)))?;
                Ok(())
            }
            (ValueType::Void, ExprValue::Void) => {
                map_builder_error(self.builder.build_return(None))?;
                Ok(())
            }
            (expected, _) => {
                bail!(
                    "return type mismatch: expected {:?}, found {:?}",
                    expected,
                    value_ty
                );
            }
        }
    }

    fn emit_error_return(&mut self, function: FunctionValue<'ctx>, ty: &ValueType) -> Result<()> {
        if let ValueType::Optional(inner) = ty {
            let nil_value = self.optional_nil(inner)?;
            if let ExprValue::Optional { value, .. } = nil_value {
                map_builder_error(self.builder.build_return(Some(&value)))?;
                return Ok(());
            }
            unreachable!("optional_nil did not produce optional value");
        }

        match function.get_type().get_return_type() {
            None => {
                map_builder_error(self.builder.build_return(None))?;
            }
            Some(basic) => {
                let zero = self.zero_value_for_basic(&basic);
                map_builder_error(self.builder.build_return(Some(&zero)))?;
            }
        }

        Ok(())
    }

    fn compile_optional_unwrap(
        &mut self,
        optional_value: StructValue<'ctx>,
        inner: Box<ValueType>,
        function: FunctionValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        let is_nil_fn = self.ensure_util_is_nil_fn();
        let is_nil_call =
            self.call_function(is_nil_fn, &[optional_value.into()], "optional_is_nil")?;
        let is_nil_raw = is_nil_call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_util_is_nil returned no value"))?
            .into_int_value();
        let is_nil_bool = self.i32_to_bool(is_nil_raw, "optional_is_nil_bool")?;

        let nil_block = self
            .context
            .append_basic_block(function, "optional_unwrap_nil");
        let ok_block = self
            .context
            .append_basic_block(function, "optional_unwrap_ok");

        map_builder_error(
            self.builder
                .build_conditional_branch(is_nil_bool, nil_block, ok_block),
        )?;

        self.builder.position_at_end(nil_block);
        let message = self.compile_string_literal("attempted to unwrap a nil value at runtime")?;
        let message_ptr = match message {
            ExprValue::String(ptr) => ptr,
            _ => unreachable!("string literal did not produce string"),
        };
        let fail_fn = self.ensure_fail_fn();
        self.call_function(fail_fn, &[message_ptr.into()], "optional_unwrap_fail")?;
        map_builder_error(self.builder.build_unreachable())?;

        self.builder.position_at_end(ok_block);
        let inner_expr = self.tea_value_to_expr(optional_value, *inner.clone())?;
        Ok(inner_expr)
    }

    fn build_coalesce(
        &mut self,
        left: &Expression,
        right: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let left_value = self.compile_expression(left, function, locals)?;
        match left_value {
            ExprValue::Optional { value, inner } => {
                let result_type = *inner.clone();
                let tmp_alloca = self.create_entry_alloca(
                    function,
                    "coalesce_tmp",
                    self.basic_type(&result_type)?,
                )?;

                let is_nil_fn = self.ensure_util_is_nil_fn();
                let is_nil_call =
                    self.call_function(is_nil_fn, &[value.into()], "coalesce_is_nil")?;
                let is_nil_raw = is_nil_call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("tea_util_is_nil returned no value"))?
                    .into_int_value();
                let is_nil_bool = self.i32_to_bool(is_nil_raw, "coalesce_is_nil_bool")?;

                let right_block = self.context.append_basic_block(function, "coalesce_rhs");
                let left_block = self.context.append_basic_block(function, "coalesce_lhs");
                let merge_block = self.context.append_basic_block(function, "coalesce_merge");

                map_builder_error(self.builder.build_conditional_branch(
                    is_nil_bool,
                    right_block,
                    left_block,
                ))?;

                self.builder.position_at_end(right_block);
                let right_value = self.compile_expression(right, function, locals)?;
                let converted_right = self
                    .convert_expr_to_type(right_value, &result_type)
                    .map_err(|error| {
                        anyhow!("right operand of '??' has incompatible type: {}", error)
                    })?;
                self.store_expr_in_pointer(
                    tmp_alloca,
                    &result_type,
                    converted_right,
                    "coalesce_tmp",
                )?;
                map_builder_error(self.builder.build_unconditional_branch(merge_block))?;

                self.builder.position_at_end(left_block);
                let left_inner = self.tea_value_to_expr(value, *inner.clone())?;
                let converted_left = self.convert_expr_to_type(left_inner, &result_type)?;
                self.store_expr_in_pointer(
                    tmp_alloca,
                    &result_type,
                    converted_left,
                    "coalesce_tmp",
                )?;
                map_builder_error(self.builder.build_unconditional_branch(merge_block))?;

                self.builder.position_at_end(merge_block);
                let temp_var = LocalVariable {
                    pointer: tmp_alloca,
                    ty: result_type.clone(),
                    mutable: true,
                };
                self.load_local_variable("coalesce_tmp", &temp_var)
            }
            ExprValue::Void => self.compile_expression(right, function, locals),
            _ => bail!("left operand of '??' must be optional"),
        }
    }

    fn expr_to_string_pointer(&mut self, value: ExprValue<'ctx>) -> Result<PointerValue<'ctx>> {
        match value {
            ExprValue::String(ptr) => Ok(ptr),
            other => {
                let tea_value = self.expr_to_tea_value(other)?;
                let func = self.ensure_util_to_string_fn();
                let call = self
                    .call_function(func, &[tea_value.into()], "tea_util_to_string")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("tea_util_to_string returned no value"))?
                    .into_pointer_value();
                Ok(call)
            }
        }
    }

    fn concat_string_values(
        &mut self,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let func = self.ensure_string_concat_fn();
        let call = self
            .call_function(func, &[left.into(), right.into()], "tea_string_concat")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_string_concat returned no value"))?
            .into_pointer_value();
        Ok(call)
    }

    fn tea_value_to_expr(
        &mut self,
        value: StructValue<'ctx>,
        ty: ValueType,
    ) -> Result<ExprValue<'ctx>> {
        match ty {
            ValueType::Int => {
                let func = self.ensure_value_as_int();
                let result = self
                    .call_function(func, &[value.into()], "val_as_int")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected Int value"))?
                    .into_int_value();
                Ok(ExprValue::Int(result))
            }
            ValueType::Float => {
                let func = self.ensure_value_as_float();
                let result = self
                    .call_function(func, &[value.into()], "val_as_float")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected Float value"))?
                    .into_float_value();
                Ok(ExprValue::Float(result))
            }
            ValueType::Bool => {
                let func = self.ensure_value_as_bool();
                let raw = self
                    .call_function(func, &[value.into()], "val_as_bool")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected Bool value"))?
                    .into_int_value();
                let bool_val = self.i32_to_bool(raw, "bool_from_i32")?;
                Ok(ExprValue::Bool(bool_val))
            }
            ValueType::String => {
                let func = self.ensure_value_as_string();
                let ptr = self
                    .call_function(func, &[value.into()], "val_as_string")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected String value"))?
                    .into_pointer_value();
                Ok(ExprValue::String(ptr))
            }
            ValueType::List(inner) => {
                let func = self.ensure_value_as_list();
                let ptr = self
                    .call_function(func, &[value.into()], "val_as_list")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected List value"))?
                    .into_pointer_value();
                Ok(ExprValue::List {
                    pointer: ptr,
                    element_type: inner,
                })
            }
            ValueType::Dict(inner) => {
                let func = self.ensure_value_as_dict();
                let ptr = self
                    .call_function(func, &[value.into()], "val_as_dict")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected Dict value"))?
                    .into_pointer_value();
                Ok(ExprValue::Dict {
                    pointer: ptr,
                    value_type: inner,
                })
            }
            ValueType::Struct(struct_name) => {
                let func = self.ensure_value_as_struct();
                let ptr = self
                    .call_function(func, &[value.into()], "val_as_struct")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected Struct value"))?
                    .into_pointer_value();
                Ok(ExprValue::Struct {
                    pointer: ptr,
                    struct_name,
                })
            }
            ValueType::Error {
                error_name,
                variant_name,
            } => {
                let func = self.ensure_value_as_error();
                let ptr = self
                    .call_function(func, &[value.into()], "val_as_error")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected Error value"))?
                    .into_pointer_value();
                Ok(ExprValue::Error {
                    pointer: ptr,
                    error_name,
                    variant_name,
                })
            }
            ValueType::Function(param_types, return_type) => {
                let func = self.ensure_value_as_closure();
                let ptr = self
                    .call_function(func, &[value.into()], "val_as_closure")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected closure value"))?
                    .into_pointer_value();
                Ok(ExprValue::Closure {
                    pointer: ptr,
                    param_types,
                    return_type,
                })
            }
            ValueType::Optional(inner) => Ok(ExprValue::Optional { value, inner }),
            ValueType::Void => Ok(ExprValue::Void),
        }
    }

    fn parse_type(&self, type_expr: &TypeExpression) -> Result<ValueType> {
        let mut repr = String::new();
        for token in &type_expr.tokens {
            repr.push_str(&token.lexeme);
        }
        self.parse_type_from_str(repr.trim())
    }

    fn parse_type_from_str(&self, repr: &str) -> Result<ValueType> {
        fn skip_ws(chars: &mut Peekable<Chars<'_>>) {
            while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
                chars.next();
            }
        }

        fn read_ident(chars: &mut Peekable<Chars<'_>>) -> Result<String> {
            skip_ws(chars);
            let mut ident = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    ident.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if ident.is_empty() {
                bail!("expected type identifier");
            }
            Ok(ident)
        }

        fn parse_inner<'a, 'ctx>(
            this: &LlvmCodeGenerator<'ctx>,
            chars: &mut Peekable<Chars<'a>>,
            structs: &HashMap<String, StructLowering<'ctx>>,
        ) -> Result<ValueType> {
            let ident = read_ident(chars)?;
            let ty_result: Result<ValueType> = match ident.as_str() {
                "Int" => Ok(ValueType::Int),
                "Float" => Ok(ValueType::Float),
                "Bool" => Ok(ValueType::Bool),
                "String" => Ok(ValueType::String),
                "Nil" => Ok(ValueType::Void),
                "Void" => Ok(ValueType::Void),
                "List" => {
                    skip_ws(chars);
                    match chars.next() {
                        Some('[') => {
                            let inner = parse_inner(this, chars, structs)?;
                            skip_ws(chars);
                            match chars.next() {
                                Some(']') => Ok(ValueType::List(Box::new(inner))),
                                _ => bail!("expected ']' to close list type"),
                            }
                        }
                        _ => bail!("expected '[' after 'List'"),
                    }
                }
                "Dict" => {
                    skip_ws(chars);
                    match chars.next() {
                        Some('[') => {}
                        _ => bail!("expected '[' after 'Dict'"),
                    }
                    let key_type = parse_inner(this, chars, structs)?;
                    skip_ws(chars);
                    match chars.next() {
                        Some(',') => {}
                        _ => bail!("expected ',' after dict key type"),
                    }
                    let value_type = parse_inner(this, chars, structs)?;
                    skip_ws(chars);
                    match chars.next() {
                        Some(']') => {}
                        _ => bail!("expected ']' to close dict type"),
                    }
                    if !matches!(key_type, ValueType::String) {
                        bail!("Dict key type must be String in LLVM backend");
                    }
                    Ok(ValueType::Dict(Box::new(value_type)))
                }
                "Func" | "Function" | "Fn" => {
                    skip_ws(chars);
                    match chars.next() {
                        Some('(') => {}
                        _ => bail!("expected '(' after function type"),
                    }

                    let mut params = Vec::new();
                    loop {
                        skip_ws(chars);
                        if matches!(chars.peek(), Some(')')) {
                            chars.next();
                            break;
                        }

                        let param_ty = parse_inner(this, chars, structs)?;
                        params.push(param_ty);
                        skip_ws(chars);
                        match chars.peek() {
                            Some(',') => {
                                chars.next();
                            }
                            Some(')') => {
                                chars.next();
                                break;
                            }
                            _ => bail!("expected ',' or ')' in function type"),
                        }
                    }

                    skip_ws(chars);
                    match (chars.next(), chars.next()) {
                        (Some('-'), Some('>')) => {}
                        _ => bail!("expected '->' in function type"),
                    }

                    let return_type = parse_inner(this, chars, structs)?;
                    Ok(ValueType::Function(params, Box::new(return_type)))
                }
                other => {
                    if let Some(mapped) = this.lookup_generic_value_type(other) {
                        Ok(mapped)
                    } else if structs.contains_key(other) {
                        Ok(ValueType::Struct(other.to_string()))
                    } else {
                        bail!("unsupported type '{other}' in LLVM backend")
                    }
                }
            };
            let mut ty = ty_result?;

            loop {
                skip_ws(chars);
                if matches!(chars.peek(), Some('?')) {
                    chars.next();
                    ty = ValueType::Optional(Box::new(ty));
                } else {
                    break;
                }
            }
            Ok(ty)
        }

        let mut chars = repr.chars().peekable();
        let ty = parse_inner(self, &mut chars, &self.structs)?;
        skip_ws(&mut chars);
        if chars.peek().is_some() {
            bail!("unexpected trailing characters in type annotation '{repr}'");
        }
        Ok(ty)
    }

    fn lookup_generic_value_type(&self, name: &str) -> Option<ValueType> {
        for scope in self.generic_binding_stack.iter().rev() {
            if let Some((_, value)) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    fn lookup_generic_type(&self, name: &str) -> Option<Type> {
        for scope in self.generic_binding_stack.iter().rev() {
            if let Some((ty, _)) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn function_type(
        &self,
        return_type: &ValueType,
        params: &[ValueType],
    ) -> Result<inkwell::types::FunctionType<'ctx>> {
        let param_types: Vec<BasicMetadataTypeEnum> = params
            .iter()
            .map(|ty| self.basic_type(ty).map(|value| value.into()))
            .collect::<Result<Vec<_>>>()?;
        let fn_type = match return_type {
            ValueType::Void => self.context.void_type().fn_type(&param_types, false),
            ValueType::Int => self.int_type().fn_type(&param_types, false),
            ValueType::Float => self.float_type().fn_type(&param_types, false),
            ValueType::Bool => self.bool_type().fn_type(&param_types, false),
            ValueType::String => self.string_ptr_type().fn_type(&param_types, false),
            ValueType::List(_) => self.list_ptr_type().fn_type(&param_types, false),
            ValueType::Dict(_) => self.dict_ptr_type().fn_type(&param_types, false),
            ValueType::Function(_, _) => self.closure_ptr_type().fn_type(&param_types, false),
            ValueType::Struct(_) => self.struct_ptr_type().fn_type(&param_types, false),
            ValueType::Error { .. } => self.error_ptr_type().fn_type(&param_types, false),
            ValueType::Optional(_) => self.value_type().fn_type(&param_types, false),
        };
        Ok(fn_type)
    }

    fn basic_type(&self, ty: &ValueType) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            ValueType::Int => Ok(self.int_type().into()),
            ValueType::Float => Ok(self.float_type().into()),
            ValueType::Bool => Ok(self.bool_type().into()),
            ValueType::String => Ok(self.string_ptr_type().into()),
            ValueType::List(_) => Ok(self.list_ptr_type().into()),
            ValueType::Dict(_) => Ok(self.dict_ptr_type().into()),
            ValueType::Function(_, _) => Ok(self.closure_ptr_type().into()),
            ValueType::Struct(_) => Ok(self.struct_ptr_type().into()),
            ValueType::Error { .. } => Ok(self.error_ptr_type().into()),
            ValueType::Optional(_) => Ok(self.value_type().into()),
            ValueType::Void => bail!("void type is not a value"),
        }
    }

    fn zero_value_for_basic(&self, ty: &BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        match ty {
            BasicTypeEnum::ArrayType(array) => array.const_zero().as_basic_value_enum(),
            BasicTypeEnum::FloatType(float) => float.const_zero().into(),
            BasicTypeEnum::IntType(int) => int.const_zero().into(),
            BasicTypeEnum::PointerType(ptr) => ptr.const_null().into(),
            BasicTypeEnum::StructType(st) => st.const_zero().as_basic_value_enum(),
            BasicTypeEnum::VectorType(vec) => vec.const_zero().as_basic_value_enum(),
        }
    }

    fn int_type(&self) -> IntType<'ctx> {
        self.context.i64_type()
    }

    fn float_type(&self) -> FloatType<'ctx> {
        self.context.f64_type()
    }

    fn bool_type(&self) -> IntType<'ctx> {
        self.context.bool_type()
    }

    fn cast_int_to_float(&self, value: IntValue<'ctx>, name: &str) -> Result<FloatValue<'ctx>> {
        map_builder_error(
            self.builder
                .build_signed_int_to_float(value, self.float_type(), name),
        )
    }

    fn bool_to_i32(&mut self, value: IntValue<'ctx>, name: &str) -> Result<IntValue<'ctx>> {
        map_builder_error(
            self.builder
                .build_int_z_extend(value, self.context.i32_type(), name),
        )
    }

    fn i32_to_bool(&mut self, value: IntValue<'ctx>, name: &str) -> Result<IntValue<'ctx>> {
        map_builder_error(
            self.builder
                .build_int_truncate(value, self.bool_type(), name),
        )
    }

    fn create_entry_alloca(
        &self,
        function: FunctionValue<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let entry = function.get_first_basic_block().expect("entry block");
        let builder = self.context.create_builder();
        match entry.get_first_instruction() {
            Some(inst) => builder.position_before(&inst),
            None => builder.position_at_end(entry),
        }
        map_builder_error(builder.build_alloca(ty, name))
    }

    fn ensure_print_int(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_int {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_int", fn_type, Some(Linkage::External));
        self.builtin_print_int = Some(func);
        func
    }

    fn ensure_print_float(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_float {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.float_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_float", fn_type, Some(Linkage::External));
        self.builtin_print_float = Some(func);
        func
    }

    fn ensure_print_bool(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_bool {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.context.i32_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_bool", fn_type, Some(Linkage::External));
        self.builtin_print_bool = Some(func);
        func
    }

    fn ensure_print_string(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_string {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_string", fn_type, Some(Linkage::External));
        self.builtin_print_string = Some(func);
        func
    }

    fn ensure_print_list(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_list {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.list_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_list", fn_type, Some(Linkage::External));
        self.builtin_print_list = Some(func);
        func
    }

    fn ensure_print_dict(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_dict {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.dict_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_dict", fn_type, Some(Linkage::External));
        self.builtin_print_dict = Some(func);
        func
    }

    fn ensure_print_struct(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_struct {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.struct_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_struct", fn_type, Some(Linkage::External));
        self.builtin_print_struct = Some(func);
        func
    }

    fn ensure_print_error(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_error {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.error_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_error", fn_type, Some(Linkage::External));
        self.builtin_print_error = Some(func);
        func
    }

    fn ensure_print_closure(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_print_closure {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.closure_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_print_closure", fn_type, Some(Linkage::External));
        self.builtin_print_closure = Some(func);
        func
    }

    fn ensure_assert_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_assert_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(
            &[
                self.context.i32_type().into(),
                self.string_ptr_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("tea_assert", fn_type, Some(Linkage::External));
        self.builtin_assert_fn = Some(func);
        func
    }

    fn ensure_assert_eq_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_assert_eq_fn {
            return func;
        }
        let value_type = self.value_type();
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[value_type.into(), value_type.into()], false);
        let func = self
            .module
            .add_function("tea_assert_eq", fn_type, Some(Linkage::External));
        self.builtin_assert_eq_fn = Some(func);
        func
    }

    fn ensure_assert_ne_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_assert_ne_fn {
            return func;
        }
        let value_type = self.value_type();
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[value_type.into(), value_type.into()], false);
        let func = self
            .module
            .add_function("tea_assert_ne", fn_type, Some(Linkage::External));
        self.builtin_assert_ne_fn = Some(func);
        func
    }

    fn ensure_fail_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_fail_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fail", fn_type, Some(Linkage::External));
        self.builtin_fail_fn = Some(func);
        func
    }

    fn ensure_util_len_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_len_fn {
            return func;
        }
        let value_type = self.value_type();
        let fn_type = self.int_type().fn_type(&[value_type.into()], false);
        let func = self
            .module
            .add_function("tea_util_len", fn_type, Some(Linkage::External));
        self.util_len_fn = Some(func);
        func
    }

    fn ensure_util_to_string_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_to_string_fn {
            return func;
        }
        let value_type = self.value_type();
        let fn_type = self.string_ptr_type().fn_type(&[value_type.into()], false);
        let func = self
            .module
            .add_function("tea_util_to_string", fn_type, Some(Linkage::External));
        self.util_to_string_fn = Some(func);
        func
    }

    fn ensure_string_concat_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.string_concat_fn {
            return func;
        }
        let ptr_type = self.string_ptr_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        let func = self
            .module
            .add_function("tea_string_concat", fn_type, Some(Linkage::External));
        self.string_concat_fn = Some(func);
        func
    }

    fn ensure_util_clamp_int_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_clamp_int_fn {
            return func;
        }
        let int_type = self.int_type();
        let fn_type = int_type.fn_type(&[int_type.into(), int_type.into(), int_type.into()], false);
        let func = self
            .module
            .add_function("tea_util_clamp_int", fn_type, Some(Linkage::External));
        self.util_clamp_int_fn = Some(func);
        func
    }

    fn ensure_util_is_nil_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_nil_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_nil", fn_type, Some(Linkage::External));
        self.util_is_nil_fn = Some(func);
        func
    }

    fn ensure_util_is_bool_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_bool_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_bool", fn_type, Some(Linkage::External));
        self.util_is_bool_fn = Some(func);
        func
    }

    fn ensure_util_is_int_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_int_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_int", fn_type, Some(Linkage::External));
        self.util_is_int_fn = Some(func);
        func
    }

    fn ensure_util_is_float_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_float_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_float", fn_type, Some(Linkage::External));
        self.util_is_float_fn = Some(func);
        func
    }

    fn ensure_util_is_string_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_string_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_string", fn_type, Some(Linkage::External));
        self.util_is_string_fn = Some(func);
        func
    }

    fn ensure_util_is_list_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_list_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_list", fn_type, Some(Linkage::External));
        self.util_is_list_fn = Some(func);
        func
    }

    fn ensure_util_is_struct_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_struct_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_struct", fn_type, Some(Linkage::External));
        self.util_is_struct_fn = Some(func);
        func
    }

    fn ensure_util_is_error_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.util_is_error_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_util_is_error", fn_type, Some(Linkage::External));
        self.util_is_error_fn = Some(func);
        func
    }

    fn ensure_env_get_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_get_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_env_get", fn_type, Some(Linkage::External));
        self.env_get_fn = Some(func);
        func
    }

    fn ensure_env_get_or_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_get_or_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.string_ptr_type().into()];
        let fn_type = self.string_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_env_get_or", fn_type, Some(Linkage::External));
        self.env_get_or_fn = Some(func);
        func
    }

    fn ensure_env_has_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_has_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_env_has", fn_type, Some(Linkage::External));
        self.env_has_fn = Some(func);
        func
    }

    fn ensure_env_require_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_require_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_env_require", fn_type, Some(Linkage::External));
        self.env_require_fn = Some(func);
        func
    }

    fn ensure_env_set_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_set_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.string_ptr_type().into()];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_env_set", fn_type, Some(Linkage::External));
        self.env_set_fn = Some(func);
        func
    }

    fn ensure_env_unset_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_unset_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_env_unset", fn_type, Some(Linkage::External));
        self.env_unset_fn = Some(func);
        func
    }

    fn ensure_env_vars_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_vars_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_env_vars", fn_type, Some(Linkage::External));
        self.env_vars_fn = Some(func);
        func
    }

    fn ensure_env_cwd_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_cwd_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_env_cwd", fn_type, Some(Linkage::External));
        self.env_cwd_fn = Some(func);
        func
    }

    fn ensure_env_set_cwd_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_set_cwd_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_env_set_cwd", fn_type, Some(Linkage::External));
        self.env_set_cwd_fn = Some(func);
        func
    }

    fn ensure_env_temp_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_temp_dir_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_env_temp_dir", fn_type, Some(Linkage::External));
        self.env_temp_dir_fn = Some(func);
        func
    }

    fn ensure_env_home_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_home_dir_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_env_home_dir", fn_type, Some(Linkage::External));
        self.env_home_dir_fn = Some(func);
        func
    }

    fn ensure_env_config_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.env_config_dir_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_env_config_dir", fn_type, Some(Linkage::External));
        self.env_config_dir_fn = Some(func);
        func
    }

    fn ensure_path_join_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_join_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.list_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_path_join", fn_type, Some(Linkage::External));
        self.path_join_fn = Some(func);
        func
    }

    fn ensure_path_components_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_components_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_path_components", fn_type, Some(Linkage::External));
        self.path_components_fn = Some(func);
        func
    }

    fn ensure_path_dirname_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_dirname_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_path_dirname", fn_type, Some(Linkage::External));
        self.path_dirname_fn = Some(func);
        func
    }

    fn ensure_path_basename_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_basename_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_path_basename", fn_type, Some(Linkage::External));
        self.path_basename_fn = Some(func);
        func
    }

    fn ensure_path_extension_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_extension_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_path_extension", fn_type, Some(Linkage::External));
        self.path_extension_fn = Some(func);
        func
    }

    fn ensure_path_set_extension_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_set_extension_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.string_ptr_type().into()];
        let fn_type = self.string_ptr_type().fn_type(&param_types, false);
        let func =
            self.module
                .add_function("tea_path_set_extension", fn_type, Some(Linkage::External));
        self.path_set_extension_fn = Some(func);
        func
    }

    fn ensure_path_strip_extension_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_strip_extension_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_path_strip_extension", fn_type, Some(Linkage::External));
        self.path_strip_extension_fn = Some(func);
        func
    }

    fn ensure_path_normalize_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_normalize_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_path_normalize", fn_type, Some(Linkage::External));
        self.path_normalize_fn = Some(func);
        func
    }

    fn ensure_path_absolute_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_absolute_fn {
            return func;
        }
        let param_types = [
            self.string_ptr_type().into(),
            self.string_ptr_type().into(),
            self.context.i32_type().into(),
        ];
        let fn_type = self.string_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_path_absolute", fn_type, Some(Linkage::External));
        self.path_absolute_fn = Some(func);
        func
    }

    fn ensure_path_relative_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_relative_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.string_ptr_type().into()];
        let fn_type = self.string_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_path_relative", fn_type, Some(Linkage::External));
        self.path_relative_fn = Some(func);
        func
    }

    fn ensure_path_is_absolute_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_is_absolute_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_path_is_absolute", fn_type, Some(Linkage::External));
        self.path_is_absolute_fn = Some(func);
        func
    }

    fn ensure_path_separator_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.path_separator_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_path_separator", fn_type, Some(Linkage::External));
        self.path_separator_fn = Some(func);
        func
    }

    fn ensure_fs_read_text_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_read_text_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_read_text", fn_type, Some(Linkage::External));
        self.fs_read_text_fn = Some(func);
        func
    }

    fn ensure_fs_write_text_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_write_text_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.string_ptr_type().into()];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_fs_write_text", fn_type, Some(Linkage::External));
        self.fs_write_text_fn = Some(func);
        func
    }

    fn ensure_fs_write_text_atomic_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_write_text_atomic_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.string_ptr_type().into()];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func =
            self.module
                .add_function("tea_fs_write_text_atomic", fn_type, Some(Linkage::External));
        self.fs_write_text_atomic_fn = Some(func);
        func
    }

    fn ensure_fs_read_bytes_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_read_bytes_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_read_bytes", fn_type, Some(Linkage::External));
        self.fs_read_bytes_fn = Some(func);
        func
    }

    fn ensure_fs_write_bytes_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_write_bytes_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.list_ptr_type().into()];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_fs_write_bytes", fn_type, Some(Linkage::External));
        self.fs_write_bytes_fn = Some(func);
        func
    }

    fn ensure_fs_write_bytes_atomic_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_write_bytes_atomic_fn {
            return func;
        }
        let param_types = [self.string_ptr_type().into(), self.list_ptr_type().into()];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func = self.module.add_function(
            "tea_fs_write_bytes_atomic",
            fn_type,
            Some(Linkage::External),
        );
        self.fs_write_bytes_atomic_fn = Some(func);
        func
    }

    fn ensure_fs_create_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_create_dir_fn {
            return func;
        }
        let param_types = [
            self.string_ptr_type().into(),
            self.context.i32_type().into(),
        ];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_fs_create_dir", fn_type, Some(Linkage::External));
        self.fs_create_dir_fn = Some(func);
        func
    }

    fn ensure_fs_ensure_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_ensure_dir_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_ensure_dir", fn_type, Some(Linkage::External));
        self.fs_ensure_dir_fn = Some(func);
        func
    }

    fn ensure_fs_ensure_parent_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_ensure_parent_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_fs_ensure_parent", fn_type, Some(Linkage::External));
        self.fs_ensure_parent_fn = Some(func);
        func
    }

    fn ensure_fs_remove_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_remove_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_remove", fn_type, Some(Linkage::External));
        self.fs_remove_fn = Some(func);
        func
    }

    fn ensure_fs_exists_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_exists_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_exists", fn_type, Some(Linkage::External));
        self.fs_exists_fn = Some(func);
        func
    }

    fn ensure_fs_is_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_is_dir_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_is_dir", fn_type, Some(Linkage::External));
        self.fs_is_dir_fn = Some(func);
        func
    }

    fn ensure_fs_is_symlink_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_is_symlink_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_is_symlink", fn_type, Some(Linkage::External));
        self.fs_is_symlink_fn = Some(func);
        func
    }

    fn ensure_fs_list_dir_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_list_dir_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_list_dir", fn_type, Some(Linkage::External));
        self.fs_list_dir_fn = Some(func);
        func
    }

    fn ensure_fs_walk_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_walk_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_walk", fn_type, Some(Linkage::External));
        self.fs_walk_fn = Some(func);
        func
    }

    fn ensure_fs_glob_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_glob_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_glob", fn_type, Some(Linkage::External));
        self.fs_glob_fn = Some(func);
        func
    }

    fn ensure_fs_size_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_size_fn {
            return func;
        }
        let fn_type = self
            .int_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_size", fn_type, Some(Linkage::External));
        self.fs_size_fn = Some(func);
        func
    }

    fn ensure_fs_modified_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_modified_fn {
            return func;
        }
        let fn_type = self
            .int_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_modified", fn_type, Some(Linkage::External));
        self.fs_modified_fn = Some(func);
        func
    }

    fn ensure_fs_permissions_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_permissions_fn {
            return func;
        }
        let fn_type = self
            .int_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_permissions", fn_type, Some(Linkage::External));
        self.fs_permissions_fn = Some(func);
        func
    }

    fn ensure_fs_is_readonly_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_is_readonly_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_is_readonly", fn_type, Some(Linkage::External));
        self.fs_is_readonly_fn = Some(func);
        func
    }

    fn ensure_fs_metadata_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_metadata_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_metadata", fn_type, Some(Linkage::External));
        self.fs_metadata_fn = Some(func);
        func
    }

    fn ensure_fs_open_read_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_open_read_fn {
            return func;
        }
        let fn_type = self
            .int_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_open_read", fn_type, Some(Linkage::External));
        self.fs_open_read_fn = Some(func);
        func
    }

    fn ensure_fs_read_chunk_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_read_chunk_fn {
            return func;
        }
        let param_types = [self.int_type().into(), self.int_type().into()];
        let fn_type = self.list_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_fs_read_chunk", fn_type, Some(Linkage::External));
        self.fs_read_chunk_fn = Some(func);
        func
    }

    fn ensure_fs_close_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.fs_close_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_fs_close", fn_type, Some(Linkage::External));
        self.fs_close_fn = Some(func);
        func
    }

    fn ensure_alloc_string(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.alloc_string_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.ptr_type.into(), self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_alloc_string", fn_type, Some(Linkage::External));
        self.alloc_string_fn = Some(func);
        func
    }

    fn ensure_alloc_list(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.alloc_list_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_alloc_list", fn_type, Some(Linkage::External));
        self.alloc_list_fn = Some(func);
        func
    }

    fn ensure_alloc_struct(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.alloc_struct_fn {
            return func;
        }
        let fn_type = self
            .struct_ptr_type()
            .fn_type(&[self.struct_template_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_alloc_struct", fn_type, Some(Linkage::External));
        self.alloc_struct_fn = Some(func);
        func
    }

    fn ensure_list_set(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.list_set_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(
            &[
                self.list_ptr_type().into(),
                self.int_type().into(),
                self.value_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("tea_list_set", fn_type, Some(Linkage::External));
        self.list_set_fn = Some(func);
        func
    }

    fn ensure_struct_set(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.struct_set_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(
            &[
                self.struct_ptr_type().into(),
                self.int_type().into(),
                self.value_type().into(),
            ],
            false,
        );
        let func =
            self.module
                .add_function("tea_struct_set_field", fn_type, Some(Linkage::External));
        self.struct_set_fn = Some(func);
        func
    }

    fn ensure_list_get(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.list_get_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(
            &[self.list_ptr_type().into(), self.int_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_list_get", fn_type, Some(Linkage::External));
        self.list_get_fn = Some(func);
        func
    }

    fn ensure_dict_new(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_new_fn {
            return func;
        }
        let fn_type = self.dict_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_dict_new", fn_type, Some(Linkage::External));
        self.dict_new_fn = Some(func);
        func
    }

    fn ensure_dict_set(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_set_fn {
            return func;
        }
        let param_types = [
            self.dict_ptr_type().into(),
            self.string_ptr_type().into(),
            self.value_type().into(),
        ];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_dict_set", fn_type, Some(Linkage::External));
        self.dict_set_fn = Some(func);
        func
    }

    fn ensure_dict_get(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_get_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(
            &[self.dict_ptr_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_dict_get", fn_type, Some(Linkage::External));
        self.dict_get_fn = Some(func);
        func
    }

    fn ensure_dict_equal(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_equal_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[self.dict_ptr_type().into(), self.dict_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_dict_equal", fn_type, Some(Linkage::External));
        self.dict_equal_fn = Some(func);
        func
    }

    fn ensure_struct_get(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.struct_get_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(
            &[self.struct_ptr_type().into(), self.int_type().into()],
            false,
        );
        let func =
            self.module
                .add_function("tea_struct_get_field", fn_type, Some(Linkage::External));
        self.struct_get_fn = Some(func);
        func
    }

    fn ensure_error_alloc(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_alloc_fn {
            return func;
        }
        let fn_type = self
            .error_ptr_type()
            .fn_type(&[self.error_template_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_alloc_error", fn_type, Some(Linkage::External));
        self.error_alloc_fn = Some(func);
        func
    }

    fn ensure_error_set(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_set_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(
            &[
                self.error_ptr_type().into(),
                self.int_type().into(),
                self.value_type().into(),
            ],
            false,
        );
        let func =
            self.module
                .add_function("tea_error_set_field", fn_type, Some(Linkage::External));
        self.error_set_fn = Some(func);
        func
    }

    fn ensure_error_get(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_get_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(
            &[self.error_ptr_type().into(), self.int_type().into()],
            false,
        );
        let func =
            self.module
                .add_function("tea_error_get_field", fn_type, Some(Linkage::External));
        self.error_get_fn = Some(func);
        func
    }

    fn ensure_error_current(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_current_fn {
            return func;
        }
        let fn_type = self.error_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_error_current", fn_type, Some(Linkage::External));
        self.error_current_fn = Some(func);
        func
    }

    fn ensure_error_set_current(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_set_current_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.error_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_error_set_current", fn_type, Some(Linkage::External));
        self.error_set_current_fn = Some(func);
        func
    }

    fn ensure_error_clear_current(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_clear_current_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(&[], false);
        let func =
            self.module
                .add_function("tea_error_clear_current", fn_type, Some(Linkage::External));
        self.error_clear_current_fn = Some(func);
        func
    }

    fn ensure_error_get_template(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.error_get_template_fn {
            return func;
        }
        let fn_type = self
            .error_template_ptr_type()
            .fn_type(&[self.error_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_error_get_template", fn_type, Some(Linkage::External));
        self.error_get_template_fn = Some(func);
        func
    }

    fn ensure_value_from_int(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_int_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_from_int", fn_type, Some(Linkage::External));
        self.value_from_int_fn = Some(func);
        func
    }

    fn ensure_value_from_float(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_float_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.float_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_float", fn_type, Some(Linkage::External));
        self.value_from_float_fn = Some(func);
        func
    }

    fn ensure_value_from_bool(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_bool_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.context.i32_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_bool", fn_type, Some(Linkage::External));
        self.value_from_bool_fn = Some(func);
        func
    }

    fn ensure_value_from_string(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_string_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_string", fn_type, Some(Linkage::External));
        self.value_from_string_fn = Some(func);
        func
    }

    fn ensure_value_from_list(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_list_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.list_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_list", fn_type, Some(Linkage::External));
        self.value_from_list_fn = Some(func);
        func
    }

    fn ensure_value_from_dict(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_dict_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.dict_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_dict", fn_type, Some(Linkage::External));
        self.value_from_dict_fn = Some(func);
        func
    }

    fn ensure_value_from_struct(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_struct_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.struct_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_struct", fn_type, Some(Linkage::External));
        self.value_from_struct_fn = Some(func);
        func
    }

    fn ensure_value_from_error(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_error_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.error_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_error", fn_type, Some(Linkage::External));
        self.value_from_error_fn = Some(func);
        func
    }

    fn ensure_value_from_closure(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_from_closure_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.closure_ptr_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_from_closure", fn_type, Some(Linkage::External));
        self.value_from_closure_fn = Some(func);
        func
    }

    fn ensure_value_nil(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_nil_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_value_nil", fn_type, Some(Linkage::External));
        self.value_nil_fn = Some(func);
        func
    }

    fn ensure_value_as_int(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_int_fn {
            return func;
        }
        let fn_type = self.int_type().fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_as_int", fn_type, Some(Linkage::External));
        self.value_as_int_fn = Some(func);
        func
    }

    fn ensure_value_as_float(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_float_fn {
            return func;
        }
        let fn_type = self
            .float_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_as_float", fn_type, Some(Linkage::External));
        self.value_as_float_fn = Some(func);
        func
    }

    fn ensure_value_as_bool(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_bool_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_as_bool", fn_type, Some(Linkage::External));
        self.value_as_bool_fn = Some(func);
        func
    }

    fn ensure_value_as_string(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_string_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_as_string", fn_type, Some(Linkage::External));
        self.value_as_string_fn = Some(func);
        func
    }

    fn ensure_value_as_list(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_list_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_as_list", fn_type, Some(Linkage::External));
        self.value_as_list_fn = Some(func);
        func
    }

    fn ensure_value_as_dict(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_dict_fn {
            return func;
        }
        let fn_type = self
            .dict_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_as_dict", fn_type, Some(Linkage::External));
        self.value_as_dict_fn = Some(func);
        func
    }

    fn ensure_value_as_struct(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_struct_fn {
            return func;
        }
        let fn_type = self
            .struct_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_as_struct", fn_type, Some(Linkage::External));
        self.value_as_struct_fn = Some(func);
        func
    }

    fn ensure_value_as_error(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_error_fn {
            return func;
        }
        let fn_type = self
            .error_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_value_as_error", fn_type, Some(Linkage::External));
        self.value_as_error_fn = Some(func);
        func
    }

    fn ensure_value_as_closure(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.value_as_closure_fn {
            return func;
        }
        let fn_type = self
            .closure_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func =
            self.module
                .add_function("tea_value_as_closure", fn_type, Some(Linkage::External));
        self.value_as_closure_fn = Some(func);
        func
    }

    fn ensure_string_equal(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.string_equal_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[self.string_ptr_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_string_equal", fn_type, Some(Linkage::External));
        self.string_equal_fn = Some(func);
        func
    }

    fn ensure_list_equal(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.list_equal_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[self.list_ptr_type().into(), self.list_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_list_equal", fn_type, Some(Linkage::External));
        self.list_equal_fn = Some(func);
        func
    }

    fn ensure_struct_equal(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.struct_equal_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[self.struct_ptr_type().into(), self.struct_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_struct_equal", fn_type, Some(Linkage::External));
        self.struct_equal_fn = Some(func);
        func
    }

    fn ensure_closure_new(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.closure_new_fn {
            return func;
        }
        let fn_type = self
            .closure_ptr_type()
            .fn_type(&[self.ptr_type.into(), self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_closure_new", fn_type, Some(Linkage::External));
        self.closure_new_fn = Some(func);
        func
    }

    fn ensure_closure_set(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.closure_set_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(
            &[
                self.closure_ptr_type().into(),
                self.int_type().into(),
                self.value_type().into(),
            ],
            false,
        );
        let func =
            self.module
                .add_function("tea_closure_set_capture", fn_type, Some(Linkage::External));
        self.closure_set_fn = Some(func);
        func
    }

    fn ensure_closure_get(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.closure_get_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(
            &[self.closure_ptr_type().into(), self.int_type().into()],
            false,
        );
        let func =
            self.module
                .add_function("tea_closure_get_capture", fn_type, Some(Linkage::External));
        self.closure_get_fn = Some(func);
        func
    }

    fn ensure_closure_equal(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.closure_equal_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[
                self.closure_ptr_type().into(),
                self.closure_ptr_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("tea_closure_equal", fn_type, Some(Linkage::External));
        self.closure_equal_fn = Some(func);
        func
    }

    fn ensure_io_read_line(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.io_read_line_fn {
            return func;
        }
        let fn_type = self.value_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_io_read_line", fn_type, Some(Linkage::External));
        self.io_read_line_fn = Some(func);
        func
    }

    fn ensure_io_read_all(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.io_read_all_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_io_read_all", fn_type, Some(Linkage::External));
        self.io_read_all_fn = Some(func);
        func
    }

    fn ensure_io_read_bytes(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.io_read_bytes_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_io_read_bytes", fn_type, Some(Linkage::External));
        self.io_read_bytes_fn = Some(func);
        func
    }

    fn ensure_io_write(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.io_write_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_io_write", fn_type, Some(Linkage::External));
        self.io_write_fn = Some(func);
        func
    }

    fn ensure_io_write_err(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.io_write_err_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_io_write_err", fn_type, Some(Linkage::External));
        self.io_write_err_fn = Some(func);
        func
    }

    fn ensure_io_flush(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.io_flush_fn {
            return func;
        }
        let fn_type = self.context.void_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_io_flush", fn_type, Some(Linkage::External));
        self.io_flush_fn = Some(func);
        func
    }

    fn ensure_cli_args_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.cli_args_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_cli_args", fn_type, Some(Linkage::External));
        self.cli_args_fn = Some(func);
        func
    }

    fn ensure_cli_parse_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.cli_parse_fn {
            return func;
        }
        let param_types = [
            self.struct_template_ptr_type().into(),
            self.value_type().into(),
            self.value_type().into(),
        ];
        let fn_type = self.struct_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_cli_parse", fn_type, Some(Linkage::External));
        self.cli_parse_fn = Some(func);
        func
    }

    fn ensure_process_run_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_run_fn {
            return func;
        }
        let param_types = [
            self.struct_template_ptr_type().into(),
            self.string_ptr_type().into(),
            self.value_type().into(),
            self.value_type().into(),
            self.value_type().into(),
            self.value_type().into(),
        ];
        let fn_type = self.struct_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_process_run", fn_type, Some(Linkage::External));
        self.process_run_fn = Some(func);
        func
    }

    fn ensure_process_spawn_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_spawn_fn {
            return func;
        }
        let param_types = [
            self.string_ptr_type().into(),
            self.value_type().into(),
            self.value_type().into(),
            self.value_type().into(),
        ];
        let fn_type = self.int_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_process_spawn", fn_type, Some(Linkage::External));
        self.process_spawn_fn = Some(func);
        func
    }

    fn ensure_process_wait_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_wait_fn {
            return func;
        }
        let param_types = [
            self.struct_template_ptr_type().into(),
            self.int_type().into(),
        ];
        let fn_type = self.struct_ptr_type().fn_type(&param_types, false);
        let func = self
            .module
            .add_function("tea_process_wait", fn_type, Some(Linkage::External));
        self.process_wait_fn = Some(func);
        func
    }

    fn ensure_process_kill_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_kill_fn {
            return func;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_process_kill", fn_type, Some(Linkage::External));
        self.process_kill_fn = Some(func);
        func
    }

    fn ensure_process_read_stdout_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_read_stdout_fn {
            return func;
        }
        let param_types = [self.int_type().into(), self.int_type().into()];
        let fn_type = self.string_ptr_type().fn_type(&param_types, false);
        let func =
            self.module
                .add_function("tea_process_read_stdout", fn_type, Some(Linkage::External));
        self.process_read_stdout_fn = Some(func);
        func
    }

    fn ensure_process_read_stderr_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_read_stderr_fn {
            return func;
        }
        let param_types = [self.int_type().into(), self.int_type().into()];
        let fn_type = self.string_ptr_type().fn_type(&param_types, false);
        let func =
            self.module
                .add_function("tea_process_read_stderr", fn_type, Some(Linkage::External));
        self.process_read_stderr_fn = Some(func);
        func
    }

    fn ensure_process_write_stdin_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_write_stdin_fn {
            return func;
        }
        let param_types = [self.int_type().into(), self.value_type().into()];
        let fn_type = self.context.void_type().fn_type(&param_types, false);
        let func =
            self.module
                .add_function("tea_process_write_stdin", fn_type, Some(Linkage::External));
        self.process_write_stdin_fn = Some(func);
        func
    }

    fn ensure_process_close_stdin_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_close_stdin_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.int_type().into()], false);
        let func =
            self.module
                .add_function("tea_process_close_stdin", fn_type, Some(Linkage::External));
        self.process_close_stdin_fn = Some(func);
        func
    }

    fn ensure_process_close_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.process_close_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_process_close", fn_type, Some(Linkage::External));
        self.process_close_fn = Some(func);
        func
    }

    fn ensure_json_encode(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.json_encode_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_json_encode", fn_type, Some(Linkage::External));
        self.json_encode_fn = Some(func);
        func
    }

    fn ensure_json_decode(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.json_decode_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_json_decode", fn_type, Some(Linkage::External));
        self.json_decode_fn = Some(func);
        func
    }

    fn ensure_yaml_encode(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.yaml_encode_fn {
            return func;
        }
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[self.value_type().into()], false);
        let func = self
            .module
            .add_function("tea_yaml_encode", fn_type, Some(Linkage::External));
        self.yaml_encode_fn = Some(func);
        func
    }

    fn ensure_yaml_decode(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.yaml_decode_fn {
            return func;
        }
        let fn_type = self
            .value_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_yaml_decode", fn_type, Some(Linkage::External));
        self.yaml_decode_fn = Some(func);
        func
    }
}

pub fn normalize_target_triple(triple: &str) -> String {
    let mut parts = triple.splitn(3, '-');
    let arch = parts.next().unwrap_or("unknown");
    let vendor = parts.next().unwrap_or("unknown");
    let rest = parts.next().unwrap_or("");

    let os = if let Some(pos) = rest.find("darwin") {
        &rest[..pos + "darwin".len()]
    } else if rest.starts_with("macos") || rest.starts_with("ios") {
        "darwin"
    } else if rest.is_empty() {
        "unknown"
    } else {
        rest
    };

    let normalized_arch = match arch {
        "arm64" => "aarch64",
        other => other,
    };

    format!("{normalized_arch}-{vendor}-{os}")
}

fn map_builder_error<T>(value: Result<T, BuilderError>) -> Result<T> {
    value.map_err(|e| anyhow!(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::normalize_target_triple;

    #[test]
    fn normalizes_arm64_darwin() {
        assert_eq!(
            normalize_target_triple("arm64-apple-darwin25.0.0"),
            "aarch64-apple-darwin"
        );
    }
}
