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
    BinaryExpression, BinaryOperator, CallExpression, CatchHandler, CatchKind,
    ConditionalExpression, ConditionalStatement, Expression, ExpressionKind, ForPattern,
    FunctionStatement, InterpolatedStringExpression, InterpolatedStringPart, LambdaBody,
    LambdaExpression, Literal, LoopHeader, LoopStatement, MatchPattern, Module as AstModule,
    ReturnStatement, SourceSpan, Statement, ThrowStatement, TryExpression, TypeExpression,
    UseStatement, VarStatement,
};
use crate::resolver::{Resolver, ResolverOutput};
use crate::stdlib::{self, StdFunctionKind};

mod helpers;
mod intrinsics;
mod types;

use crate::typechecker::{
    ErrorDefinition, FunctionInstance, StructDefinition, StructInstance, StructType, Type,
    TypeChecker,
};
use helpers::{add_function_attr, build_tea_value, LoopMetadataBuilder, TeaValueTag};
use intrinsics::Intrinsic;
use types::{
    format_struct_type_name, mangle_function_name, sanitize_symbol_component, type_to_value_type,
    ErrorHandlingMode, ErrorVariantLowering, ExprValue, FunctionSignature, GlobalBindingSlot,
    LambdaSignature, LocalVariable, StructLowering, ValueType,
};

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
    pub lto: bool,
}

impl<'a> Default for ObjectCompileOptions<'a> {
    fn default() -> Self {
        Self {
            triple: None,
            cpu: None,
            features: None,
            opt_level: OptimizationLevel::Aggressive, // O3 by default for maximum performance
            entry_symbol: None,
            lto: false, // LTO disabled by default (slower compile time)
        }
    }
}

/// Run LLVM optimization passes on the module using the opt tool
/// This is CRITICAL for function inlining to work with alwaysinline attribute
fn optimize_module_with_opt<'ctx>(
    context: &'ctx Context,
    module: LlvmModule<'ctx>,
    opt_level: OptimizationLevel,
) -> Result<LlvmModule<'ctx>> {
    // Skip optimization for level 0
    if matches!(opt_level, OptimizationLevel::None) {
        return Ok(module);
    }

    // Determine the opt level flag
    let opt_flag = match opt_level {
        OptimizationLevel::None => return Ok(module),
        OptimizationLevel::Less => "-O1",
        OptimizationLevel::Default => "-O2",
        OptimizationLevel::Aggressive => "-O3",
    };

    // Get the IR as a string
    let ir_string = module.print_to_string().to_string();

    // Try to find the opt tool
    // Use LLVM 17 to match inkwell's version
    let opt_paths = [
        "/opt/homebrew/opt/llvm@17/bin/opt", // Homebrew LLVM 17 on Apple Silicon (matches inkwell)
        "/usr/local/opt/llvm@17/bin/opt",    // Homebrew LLVM 17 on Intel Mac
        "/opt/homebrew/opt/llvm/bin/opt",    // Homebrew on Apple Silicon (fallback)
        "/usr/local/opt/llvm/bin/opt",       // Homebrew on Intel Mac (fallback)
        "/usr/bin/opt",                      // Linux system install
        "opt",                               // In PATH
    ];

    let mut opt_path = None;
    for path in &opt_paths {
        if Command::new(path).arg("--version").output().is_ok() {
            opt_path = Some(*path);
            break;
        }
    }

    let opt_tool = match opt_path {
        Some(path) => path,
        None => {
            // If opt tool is not found, just return the module
            // WARNING: Without opt, function inlining (alwaysinline) will NOT work!
            // This will result in significantly slower binaries.
            eprintln!("Warning: LLVM opt tool not found. Function inlining disabled.");
            eprintln!("Install LLVM 17 for optimal performance: brew install llvm@17");
            return Ok(module);
        }
    };

    // Run opt on the IR to perform optimizations including function inlining
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(opt_tool)
        .arg(opt_flag)
        .arg("-S") // Output textual IR
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn opt process at {}", opt_tool))?;

    // Write IR to stdin and explicitly close it
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(ir_string.as_bytes())
            .context("failed to write IR to opt stdin")?;
        // stdin is dropped here, closing the pipe
    } else {
        bail!("failed to open stdin for opt");
    }

    let output = child
        .wait_with_output()
        .context("failed to wait for opt process")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("opt failed: {}", stderr);
    }

    // Parse the optimized IR back into a module
    let mut optimized_ir =
        String::from_utf8(output.stdout).context("opt output was not valid UTF-8")?;

    // Ensure the IR ends with a newline to avoid parsing issues
    if !optimized_ir.ends_with('\n') {
        optimized_ir.push('\n');
    }

    let memory_buffer = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range(
        optimized_ir.as_bytes(),
        "optimized_ir",
    );

    context
        .create_module_from_ir(memory_buffer)
        .map_err(|e| anyhow!("failed to parse optimized IR: {}", e))
}

pub fn compile_source_to_object(
    source_path: &std::path::Path,
    output_path: &std::path::Path,
    options: &ObjectCompileOptions<'_>,
) -> Result<()> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::source::{SourceFile, SourceId};
    use std::fs;

    let source_code = fs::read_to_string(source_path)
        .with_context(|| format!("failed to read source file: {}", source_path.display()))?;

    let source = SourceFile::new(SourceId(0), source_path.to_path_buf(), source_code);
    let mut lexer = Lexer::new(&source)?;
    let tokens = lexer.tokenize()?;

    let mut parser = Parser::new(&source, tokens);
    let module_ast = parser.parse()?;

    compile_module_to_object(&module_ast, output_path, options)
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

    // Run LLVM IR optimizations
    let module = optimize_module_with_opt(&context, module, options.opt_level)?;

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

    // Always emit regular object files
    // Note: Function inlining with alwaysinline attribute happens during code generation
    // at the optimization level specified. LLVM 17+ moved to a new PassManager API
    // which is more complex. For now, we rely on the optimization level set on target_machine.
    target_machine
        .write_to_file(&module, FileType::Object, output_path)
        .map_err(|e| anyhow!(format!("failed to write object file: {e}")))
}

/// Context for compiling break/continue within loops
struct LoopContext<'ctx> {
    /// Block to jump to for `continue` (increment/condition block)
    continue_block: inkwell::basic_block::BasicBlock<'ctx>,
    /// Block to jump to for `break` (exit block)
    exit_block: inkwell::basic_block::BasicBlock<'ctx>,
}

// Many fields are for removed functionality but kept for potential future use
#[allow(dead_code)]
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
    builtin_println_int: Option<FunctionValue<'ctx>>,
    builtin_println_float: Option<FunctionValue<'ctx>>,
    builtin_println_bool: Option<FunctionValue<'ctx>>,
    builtin_println_string: Option<FunctionValue<'ctx>>,
    builtin_println_list: Option<FunctionValue<'ctx>>,
    builtin_println_dict: Option<FunctionValue<'ctx>>,
    builtin_println_closure: Option<FunctionValue<'ctx>>,
    builtin_println_struct: Option<FunctionValue<'ctx>>,
    builtin_println_error: Option<FunctionValue<'ctx>>,
    builtin_type_of_fn: Option<FunctionValue<'ctx>>,
    builtin_panic_fn: Option<FunctionValue<'ctx>>,
    builtin_exit_fn: Option<FunctionValue<'ctx>>,
    builtin_dict_delete_fn: Option<FunctionValue<'ctx>>,
    builtin_dict_clear_fn: Option<FunctionValue<'ctx>>,
    builtin_fmax_fn: Option<FunctionValue<'ctx>>,
    builtin_fmin_fn: Option<FunctionValue<'ctx>>,
    builtin_list_append_fn: Option<FunctionValue<'ctx>>,
    builtin_assert_fn: Option<FunctionValue<'ctx>>,
    builtin_assert_eq_fn: Option<FunctionValue<'ctx>>,
    builtin_assert_ne_fn: Option<FunctionValue<'ctx>>,
    builtin_fail_fn: Option<FunctionValue<'ctx>>,
    util_len_fn: Option<FunctionValue<'ctx>>,
    util_to_string_fn: Option<FunctionValue<'ctx>>,
    malloc_fn: Option<FunctionValue<'ctx>>,
    memcpy_fn: Option<FunctionValue<'ctx>>,
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
    args_program_fn: Option<FunctionValue<'ctx>>,
    // Regex functions
    regex_compile_fn: Option<FunctionValue<'ctx>>,
    regex_is_match_fn: Option<FunctionValue<'ctx>>,
    regex_find_all_fn: Option<FunctionValue<'ctx>>,
    regex_captures_fn: Option<FunctionValue<'ctx>>,
    regex_replace_fn: Option<FunctionValue<'ctx>>,
    regex_replace_all_fn: Option<FunctionValue<'ctx>>,
    regex_split_fn: Option<FunctionValue<'ctx>>,
    read_line_fn: Option<FunctionValue<'ctx>>,
    read_all_fn: Option<FunctionValue<'ctx>>,
    eprint_int_fn: Option<FunctionValue<'ctx>>,
    eprint_float_fn: Option<FunctionValue<'ctx>>,
    eprint_bool_fn: Option<FunctionValue<'ctx>>,
    eprint_string_fn: Option<FunctionValue<'ctx>>,
    eprint_list_fn: Option<FunctionValue<'ctx>>,
    eprint_dict_fn: Option<FunctionValue<'ctx>>,
    eprint_struct_fn: Option<FunctionValue<'ctx>>,
    eprint_error_fn: Option<FunctionValue<'ctx>>,
    eprint_closure_fn: Option<FunctionValue<'ctx>>,
    eprintln_int_fn: Option<FunctionValue<'ctx>>,
    eprintln_float_fn: Option<FunctionValue<'ctx>>,
    eprintln_bool_fn: Option<FunctionValue<'ctx>>,
    eprintln_string_fn: Option<FunctionValue<'ctx>>,
    eprintln_list_fn: Option<FunctionValue<'ctx>>,
    eprintln_dict_fn: Option<FunctionValue<'ctx>>,
    eprintln_struct_fn: Option<FunctionValue<'ctx>>,
    eprintln_error_fn: Option<FunctionValue<'ctx>>,
    eprintln_closure_fn: Option<FunctionValue<'ctx>>,
    is_tty_fn: Option<FunctionValue<'ctx>>,
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
    string_index_fn: Option<FunctionValue<'ctx>>,
    list_concat_fn: Option<FunctionValue<'ctx>>,
    string_concat_fn: Option<FunctionValue<'ctx>>,
    string_push_str_fn: Option<FunctionValue<'ctx>>,
    string_slice_fn: Option<FunctionValue<'ctx>>,
    list_slice_fn: Option<FunctionValue<'ctx>>,
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
    error_mode_stack: Vec<ErrorHandlingMode>,
    function_return_stack: Vec<ValueType>,
    function_can_throw_stack: Vec<bool>,
    loop_context: Option<LoopContext<'ctx>>,
    list_len_ffi_fn: Option<FunctionValue<'ctx>>,
    dict_keys_fn: Option<FunctionValue<'ctx>>,
    dict_values_fn: Option<FunctionValue<'ctx>>,
    dict_entries_fn: Option<FunctionValue<'ctx>>,
}

/// Macros to generate FFI helper functions.
/// Each invocation generates a method that lazily initializes an external function declaration.

/// FFI for type checking utilities: (value) -> i32
macro_rules! define_ffi_typecheck_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .i32_type()
                .fn_type(&[self.value_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for no-param string getters: () -> string
macro_rules! define_ffi_string_getter_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self.string_ptr_type().fn_type(&[], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for string transforms: (string) -> string
macro_rules! define_ffi_string_transform_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .string_ptr_type()
                .fn_type(&[self.string_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for string predicates: (string) -> i32 (bool)
macro_rules! define_ffi_string_predicate_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .i32_type()
                .fn_type(&[self.string_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for string setters: (string) -> void
macro_rules! define_ffi_string_setter_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.string_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for two-string functions: (string, string) -> string
macro_rules! define_ffi_string2_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self.string_ptr_type().fn_type(
                &[self.string_ptr_type().into(), self.string_ptr_type().into()],
                false,
            );
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for list to string: (list) -> string
macro_rules! define_ffi_list_to_string_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .string_ptr_type()
                .fn_type(&[self.list_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for string to list: (string) -> list
macro_rules! define_ffi_string_to_list_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .list_ptr_type()
                .fn_type(&[self.string_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for string to int functions: (string) -> int
macro_rules! define_ffi_string_to_int_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .int_type()
                .fn_type(&[self.string_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// FFI for print operations with various types
macro_rules! define_ffi_print_fn {
    ($fn_name:ident, $field:ident, $ffi_name:literal, int) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.int_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, float) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.float_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, bool) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.context.i32_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, string) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.string_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, list) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.list_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, dict) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.dict_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, struct) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.struct_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, error) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.error_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
    ($fn_name:ident, $field:ident, $ffi_name:literal, closure) => {
        fn $fn_name(&mut self) -> FunctionValue<'ctx> {
            if let Some(func) = self.$field {
                return func;
            }
            let fn_type = self
                .context
                .void_type()
                .fn_type(&[self.closure_ptr_type().into()], false);
            let func = self
                .module
                .add_function($ffi_name, fn_type, Some(Linkage::External));
            self.$field = Some(func);
            func
        }
    };
}

/// Macro for no-arg builtin calls that return a string
macro_rules! compile_noarg_string_call {
    ($fn_name:ident, $ensure_fn:ident, $ffi_name:literal, $builtin_name:literal) => {
        fn $fn_name(
            &mut self,
            arguments: &[crate::ast::CallArgument],
            _function: FunctionValue<'ctx>,
            _locals: &mut HashMap<String, LocalVariable<'ctx>>,
        ) -> Result<ExprValue<'ctx>> {
            if !arguments.is_empty() {
                bail!(concat!($builtin_name, " expects no arguments"));
            }
            let func = self.$ensure_fn();
            let pointer = self
                .call_function(func, &[], $ffi_name)?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!(concat!($ffi_name, " returned no value")))?
                .into_pointer_value();
            Ok(ExprValue::String(pointer))
        }
    };
}

/// Macro for single-string-arg builtin calls that return a string
macro_rules! compile_string_to_string_call {
    ($fn_name:ident, $ensure_fn:ident, $ffi_name:literal, $builtin_name:literal) => {
        fn $fn_name(
            &mut self,
            arguments: &[crate::ast::CallArgument],
            function: FunctionValue<'ctx>,
            locals: &mut HashMap<String, LocalVariable<'ctx>>,
        ) -> Result<ExprValue<'ctx>> {
            if arguments.len() != 1 {
                bail!(concat!($builtin_name, " expects exactly 1 argument"));
            }
            if arguments[0].name.is_some() {
                bail!(concat!(
                    "named arguments are not supported for ",
                    $builtin_name
                ));
            }
            let arg_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
            let arg_ptr = match arg_expr {
                ExprValue::String(ptr) => ptr,
                _ => bail!(concat!($builtin_name, " expects a String argument")),
            };
            let func = self.$ensure_fn();
            let pointer = self
                .call_function(func, &[arg_ptr.into()], $ffi_name)?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!(concat!($ffi_name, " returned no value")))?
                .into_pointer_value();
            Ok(ExprValue::String(pointer))
        }
    };
}

// Many methods are for removed functionality but kept for potential future use
#[allow(dead_code)]
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
        let i8 = context.i8_type();
        let ptr_type = context.ptr_type(AddressSpace::default());

        // Small string optimization: 24-byte struct
        // tag (i8): 0=heap, 1=inline
        // len (i8): length for inline strings (0-22)
        // data ([22 x i8]): inline string data OR heap pointer in first 8 bytes
        tea_string.set_body(
            &[
                i8.into(),                // tag
                i8.into(),                // len (for inline) or padding (for heap)
                i8.array_type(22).into(), // inline data or pointer storage
            ],
            false,
        );

        // TeaValue = { tag: i32, _padding: i32, payload: i64 }
        // Explicit padding to ensure consistent C ABI layout (16 bytes total)
        let value_payload = context.i64_type();
        tea_value.set_body(
            &[
                context.i32_type().into(), // tag
                context.i32_type().into(), // explicit padding for alignment
                value_payload.into(),      // payload (i64)
            ],
            false,
        );

        // Small list optimization: 136-byte struct
        // tag (i8): 0=heap, 1=inline
        // len (i8): length for inline lists (0-7) or padding for heap
        // padding ([6 x i8]): alignment
        // data ([8 x TeaValue]): inline items OR first 24 bytes hold heap info
        tea_list.set_body(
            &[
                i8.into(),                      // tag
                i8.into(),                      // len (for inline) or padding (for heap)
                i8.array_type(6).into(),        // padding for alignment
                tea_value.array_type(8).into(), // inline items or heap storage
            ],
            false,
        );
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

        // Register global built-in functions
        let mut builtin_functions = HashMap::new();
        for builtin in stdlib::BUILTINS {
            builtin_functions.insert(builtin.name.to_string(), builtin.kind);
        }

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
            builtin_functions,
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
            builtin_println_int: None,
            builtin_println_float: None,
            builtin_println_bool: None,
            builtin_println_string: None,
            builtin_println_list: None,
            builtin_println_dict: None,
            builtin_println_closure: None,
            builtin_println_struct: None,
            builtin_println_error: None,
            builtin_type_of_fn: None,
            builtin_panic_fn: None,
            builtin_exit_fn: None,
            builtin_dict_delete_fn: None,
            builtin_dict_clear_fn: None,
            builtin_fmax_fn: None,
            builtin_fmin_fn: None,
            builtin_list_append_fn: None,
            builtin_assert_fn: None,
            builtin_assert_eq_fn: None,
            builtin_assert_ne_fn: None,
            builtin_fail_fn: None,
            util_len_fn: None,
            util_to_string_fn: None,
            malloc_fn: None,
            memcpy_fn: None,
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
            args_program_fn: None,
            // Regex functions
            regex_compile_fn: None,
            regex_is_match_fn: None,
            regex_find_all_fn: None,
            regex_captures_fn: None,
            regex_replace_fn: None,
            regex_replace_all_fn: None,
            regex_split_fn: None,
            read_line_fn: None,
            read_all_fn: None,
            eprint_int_fn: None,
            eprint_float_fn: None,
            eprint_bool_fn: None,
            eprint_string_fn: None,
            eprint_list_fn: None,
            eprint_dict_fn: None,
            eprint_struct_fn: None,
            eprint_error_fn: None,
            eprint_closure_fn: None,
            eprintln_int_fn: None,
            eprintln_float_fn: None,
            eprintln_bool_fn: None,
            eprintln_string_fn: None,
            eprintln_list_fn: None,
            eprintln_dict_fn: None,
            eprintln_struct_fn: None,
            eprintln_error_fn: None,
            eprintln_closure_fn: None,
            is_tty_fn: None,
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
            string_index_fn: None,
            list_concat_fn: None,
            string_concat_fn: None,
            string_push_str_fn: None,
            string_slice_fn: None,
            list_slice_fn: None,
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
            error_current_fn: None,
            error_set_current_fn: None,
            error_clear_current_fn: None,
            error_get_template_fn: None,
            error_mode_stack: vec![ErrorHandlingMode::Propagate],
            function_return_stack: Vec::new(),
            function_can_throw_stack: Vec::new(),
            global_slots: HashMap::new(),
            loop_context: None,
            list_len_ffi_fn: None,
            dict_keys_fn: None,
            dict_values_fn: None,
            dict_entries_fn: None,
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

    fn push_function_can_throw(&mut self, can_throw: bool) {
        self.function_can_throw_stack.push(can_throw);
    }

    fn pop_function_can_throw(&mut self) {
        self.function_can_throw_stack
            .pop()
            .expect("function can_throw stack underflow");
    }

    fn current_function_can_throw(&self) -> bool {
        self.function_can_throw_stack
            .last()
            .copied()
            .unwrap_or(false)
    }

    /// Check if a function is small enough for aggressive inlining
    fn is_small_function(&self, statements: &[Statement]) -> bool {
        // Count statements recursively
        // Increased threshold to 20 for more aggressive inlining
        // This helps with functions like sum_to_n in benchmarks
        self.count_statements(statements) < 20
    }

    /// Count total statements including nested blocks
    fn count_statements(&self, statements: &[Statement]) -> usize {
        let mut count = 0;
        for stmt in statements {
            count += 1;
            match stmt {
                Statement::Conditional(cond_stmt) => {
                    count += self.count_statements(&cond_stmt.consequent.statements);
                    if let Some(alternative) = &cond_stmt.alternative {
                        count += self.count_statements(&alternative.statements);
                    }
                }
                Statement::Loop(loop_stmt) => {
                    count += self.count_statements(&loop_stmt.body.statements);
                }
                Statement::Match(match_stmt) => {
                    for arm in &match_stmt.arms {
                        count += self.count_statements(&arm.block.statements);
                    }
                }
                _ => {}
            }
        }
        count
    }

    /// Add optimization attributes to a function
    fn add_function_attributes(
        &self,
        function: FunctionValue<'ctx>,
        can_throw: bool,
        is_small: bool,
    ) {
        // Add nounwind if the function doesn't throw
        if !can_throw {
            add_function_attr(&self.context, function, "nounwind");
        }

        // Add willreturn - all our functions return (no infinite loops exposed to LLVM)
        add_function_attr(&self.context, function, "willreturn");

        // Add nosync - Tea functions don't use synchronization primitives
        // This helps LLVM understand that functions can be safely parallelized/vectorized
        add_function_attr(&self.context, function, "nosync");

        // Add nofree - Tea functions use RAII and don't call free() directly
        // This helps with alias analysis for vectorization
        add_function_attr(&self.context, function, "nofree");

        // Add aggressive inlining for small functions
        // Use alwaysinline instead of inlinehint for guaranteed inlining
        // This is crucial for performance in tight loops (e.g., sum_to_n called 100k times)
        if is_small {
            add_function_attr(&self.context, function, "alwaysinline");
        }
    }

    /// Add loop optimization metadata to help LLVM vectorize and optimize loops
    fn add_loop_metadata(&self, instruction: inkwell::values::InstructionValue<'ctx>) {
        LoopMetadataBuilder::new(&self.context, self.bool_type())
            .with_bool("llvm.loop.vectorize.enable", true)
            .with_i32("llvm.loop.vectorize.width", 4)
            .with_bool("llvm.loop.vectorize.scalable.enable", false)
            .with_i32("llvm.loop.interleave.count", 4)
            .with_bool("llvm.loop.unroll.enable", true)
            .attach_to(instruction);
    }

    /// Analyze which parameters are mutated in the function body
    fn find_mutated_parameters(
        &self,
        function: &FunctionStatement,
    ) -> std::collections::HashSet<String> {
        use std::collections::HashSet;
        let mut mutated = HashSet::new();
        self.find_mutated_in_statements(&function.body.statements, &mut mutated);
        mutated
    }

    /// Recursively find assignments to identifiers in statements
    fn find_mutated_in_statements(
        &self,
        statements: &[Statement],
        mutated: &mut std::collections::HashSet<String>,
    ) {
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) => {
                    self.find_mutated_in_expression(&expr.expression, mutated);
                }
                Statement::Conditional(cond) => {
                    self.find_mutated_in_statements(&cond.consequent.statements, mutated);
                    if let Some(alt) = &cond.alternative {
                        self.find_mutated_in_statements(&alt.statements, mutated);
                    }
                }
                Statement::Loop(loop_stmt) => {
                    self.find_mutated_in_statements(&loop_stmt.body.statements, mutated);
                }
                Statement::Match(match_stmt) => {
                    for arm in &match_stmt.arms {
                        self.find_mutated_in_statements(&arm.block.statements, mutated);
                    }
                }
                Statement::Return(ret) => {
                    if let Some(expr) = &ret.expression {
                        self.find_mutated_in_expression(expr, mutated);
                    }
                }
                Statement::Throw(throw) => {
                    self.find_mutated_in_expression(&throw.expression, mutated);
                }
                Statement::Var(var) => {
                    for binding in &var.bindings {
                        if let Some(init) = &binding.initializer {
                            self.find_mutated_in_expression(init, mutated);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Recursively find assignments in expressions
    fn find_mutated_in_expression(
        &self,
        expr: &Expression,
        mutated: &mut std::collections::HashSet<String>,
    ) {
        match &expr.kind {
            ExpressionKind::Assignment(assignment) => {
                // Check if the assignment target is an identifier
                if let ExpressionKind::Identifier(ident) = &assignment.target.kind {
                    mutated.insert(ident.name.clone());
                }
                // Also check nested expressions
                self.find_mutated_in_expression(&assignment.value, mutated);
            }
            ExpressionKind::Binary(binary) => {
                self.find_mutated_in_expression(&binary.left, mutated);
                self.find_mutated_in_expression(&binary.right, mutated);
            }
            ExpressionKind::Unary(unary) => {
                self.find_mutated_in_expression(&unary.operand, mutated);
            }
            ExpressionKind::Call(call) => {
                self.find_mutated_in_expression(&call.callee, mutated);
                for arg in &call.arguments {
                    self.find_mutated_in_expression(&arg.expression, mutated);
                }
            }
            ExpressionKind::Conditional(cond) => {
                self.find_mutated_in_expression(&cond.condition, mutated);
                self.find_mutated_in_expression(&cond.consequent, mutated);
                self.find_mutated_in_expression(&cond.alternative, mutated);
            }
            ExpressionKind::Lambda(lambda) => {
                // Lambda body could mutate captures, but we don't track that here
                match &lambda.body {
                    LambdaBody::Block(block) => {
                        self.find_mutated_in_statements(&block.statements, mutated);
                    }
                    LambdaBody::Expression(expr) => {
                        self.find_mutated_in_expression(expr, mutated);
                    }
                }
            }
            ExpressionKind::Try(try_expr) => {
                self.find_mutated_in_expression(&try_expr.expression, mutated);
                if let Some(catch) = &try_expr.catch {
                    match &catch.kind {
                        CatchKind::Fallback(fallback) => {
                            self.find_mutated_in_expression(fallback, mutated);
                        }
                        CatchKind::Arms(arms) => {
                            for arm in arms {
                                match &arm.handler {
                                    CatchHandler::Expression(expr) => {
                                        self.find_mutated_in_expression(expr, mutated);
                                    }
                                    CatchHandler::Block(block) => {
                                        self.find_mutated_in_statements(&block.statements, mutated);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ExpressionKind::Match(match_expr) => {
                self.find_mutated_in_expression(&match_expr.scrutinee, mutated);
                for arm in &match_expr.arms {
                    self.find_mutated_in_expression(&arm.expression, mutated);
                }
            }
            ExpressionKind::List(list) => {
                for elem in &list.elements {
                    self.find_mutated_in_expression(elem, mutated);
                }
            }
            ExpressionKind::Dict(dict) => {
                for entry in &dict.entries {
                    self.find_mutated_in_expression(&entry.value, mutated);
                }
            }
            ExpressionKind::InterpolatedString(interp) => {
                for part in &interp.parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.find_mutated_in_expression(expr, mutated);
                    }
                }
            }
            _ => {}
        }
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
        // Only clear error state if the current function can throw
        if self.current_function_can_throw() {
            let clear_fn = self.ensure_error_clear_current();
            self.call_function(clear_fn, &[], "error_clear")?;
        }
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
        // Pre-register builtin structs from typechecker so they're available during type parsing
        self.register_builtin_structs()?;
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

    /// Pre-register builtin structs from the typechecker so they're available during type parsing.
    /// This handles structs like ProcessResult that are not defined in user code.
    /// Structs with Unknown types in their fields are skipped (e.g., CliParseResult).
    fn register_builtin_structs(&mut self) -> Result<()> {
        // Get the names of all builtin structs from struct_definitions_tc that aren't in the AST
        let builtin_struct_names: Vec<String> = self
            .struct_definitions_tc
            .keys()
            .filter(|name| !self.structs.contains_key(*name))
            .cloned()
            .collect();

        for name in builtin_struct_names {
            if let Some(definition) = self.struct_definitions_tc.get(&name) {
                if definition.type_parameters.is_empty() {
                    // Try to lower all field types - skip this struct if any fail
                    let mut lowered_types = Vec::with_capacity(definition.fields.len());
                    let mut can_lower = true;
                    for field in &definition.fields {
                        match type_to_value_type(&field.ty) {
                            Ok(ty) => lowered_types.push(ty),
                            Err(_) => {
                                // Skip structs with Unknown or other non-lowerable types
                                can_lower = false;
                                break;
                            }
                        }
                    }

                    if can_lower {
                        let mut lowering = StructLowering::new();
                        lowering.field_names = definition
                            .fields
                            .iter()
                            .map(|field| field.name.clone())
                            .collect();
                        lowering.field_types = lowered_types.clone();

                        self.struct_field_variants
                            .entry(name.clone())
                            .or_insert_with(|| lowered_types.clone());
                        self.struct_variant_bases
                            .entry(name.clone())
                            .or_insert_with(|| name.clone());
                        self.structs.insert(name, lowering);
                    }
                }
            }
        }
        Ok(())
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

            // Add optimization attributes
            let is_small = self.is_small_function(&function.body.statements);
            self.add_function_attributes(fn_value, can_throw, is_small);

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

            // Add optimization attributes
            let is_small = self.is_small_function(&function.body.statements);
            self.add_function_attributes(fn_value, can_throw, is_small);

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
        self.push_function_can_throw(signature.can_throw);

        // Analyze which parameters are mutated
        let mutated_params = self.find_mutated_parameters(function);

        let mut locals: HashMap<String, LocalVariable<'ctx>> = HashMap::new();
        for (index, param) in function.parameters.iter().enumerate() {
            let arg = signature.value.get_nth_param(index as u32).expect("param");
            arg.set_name(&param.name);
            let param_type = signature.param_types[index].clone();

            let is_mutable = mutated_params.contains(&param.name);

            if is_mutable {
                // Mutable parameter: allocate on stack
                let alloca = self.create_entry_alloca(
                    signature.value,
                    &param.name,
                    self.basic_type(&param_type)?,
                )?;
                map_builder_error(self.builder.build_store(alloca, arg))?;
                locals.insert(
                    param.name.clone(),
                    LocalVariable {
                        pointer: Some(alloca),
                        value: None,
                        ty: param_type,
                        mutable: true,
                    },
                );
            } else {
                // Immutable parameter: keep in SSA register
                locals.insert(
                    param.name.clone(),
                    LocalVariable {
                        pointer: None,
                        value: Some(arg),
                        ty: param_type,
                        mutable: false,
                    },
                );
            }
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
        self.pop_function_can_throw();

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
        self.push_function_can_throw(false); // main doesn't throw
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
        self.pop_function_can_throw();

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
            Statement::Break(_) => {
                if let Some(ctx) = &self.loop_context {
                    map_builder_error(self.builder.build_unconditional_branch(ctx.exit_block))?;
                    Ok(true) // terminated
                } else {
                    bail!("break outside of loop")
                }
            }
            Statement::Continue(_) => {
                if let Some(ctx) = &self.loop_context {
                    map_builder_error(self.builder.build_unconditional_branch(ctx.continue_block))?;
                    Ok(true) // terminated
                } else {
                    bail!("continue outside of loop")
                }
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
                initial_value.clone(),
                &binding.name,
            )?;
            if let Some(slot) = self.global_slots.get_mut(&binding.name) {
                slot.initialized = true;
                slot.mutable = !statement.is_const;
                slot.ty = ty.clone();
            }

            // Optimization: For const globals, keep the value in SSA form instead of pointer
            // This avoids loading from memory on every access
            if statement.is_const {
                if let Some(basic_value) = initial_value.into_basic_value() {
                    locals.insert(
                        binding.name.clone(),
                        LocalVariable {
                            pointer: None,
                            value: Some(basic_value),
                            ty,
                            mutable: false,
                        },
                    );
                } else {
                    // Void type - still need to track but no value
                    locals.insert(
                        binding.name.clone(),
                        LocalVariable {
                            pointer: None,
                            value: None,
                            ty,
                            mutable: false,
                        },
                    );
                }
            } else {
                // Mutable globals need the pointer for assignments
                locals.insert(
                    binding.name.clone(),
                    LocalVariable {
                        pointer: Some(global.as_pointer_value()),
                        value: None,
                        ty,
                        mutable: true,
                    },
                );
            }
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

            let is_mutable = !statement.is_const;

            // Optimization: Use SSA values for immutable variables (no memory allocation)
            if !is_mutable {
                // For const variables, store the SSA value directly
                if let Some(basic_value) = initial_value.clone().into_basic_value() {
                    locals.insert(
                        binding.name.clone(),
                        LocalVariable {
                            pointer: None,
                            value: Some(basic_value),
                            ty,
                            mutable: false,
                        },
                    );
                } else {
                    // Void type - still store in locals for name tracking
                    locals.insert(
                        binding.name.clone(),
                        LocalVariable {
                            pointer: None,
                            value: None,
                            ty,
                            mutable: false,
                        },
                    );
                }
            } else {
                // For mutable variables, allocate stack space as before
                let alloca =
                    self.create_entry_alloca(function, &binding.name, self.basic_type(&ty)?)?;
                self.store_expr_in_pointer(alloca, &ty, initial_value, &binding.name)?;
                locals.insert(
                    binding.name.clone(),
                    LocalVariable {
                        pointer: Some(alloca),
                        value: None,
                        ty,
                        mutable: true,
                    },
                );
            }
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
                // Check if this is a tail call (directly returning a call result)
                let is_tail_call = matches!(&expr.kind, ExpressionKind::Call(_));

                if is_tail_call {
                    // Compile the call with tail call hint
                    if let ExpressionKind::Call(call) = &expr.kind {
                        let value = self
                            .compile_call_with_tail_hint(call, expr.span, function, locals, true)?;
                        let converted = self.convert_expr_to_type(value, ty)?;
                        return self.emit_return_value(converted, ty);
                    }
                }

                // Normal return (not a tail call)
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
        let condition = self
            .compile_expression(&statement.condition, function, locals)?
            .into_bool()?;

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
            // Both branches terminated, but the merge block still exists and needs a terminator
            self.builder.position_at_end(merge_block);
            map_builder_error(self.builder.build_unreachable())?;
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
        match &statement.header {
            LoopHeader::Condition(expr) => {
                self.compile_while_loop(statement, expr, function, locals, return_type)
            }
            LoopHeader::For { pattern, iterator } => {
                self.compile_for_loop(statement, pattern, iterator, function, locals, return_type)
            }
        }
    }

    fn compile_while_loop(
        &mut self,
        statement: &LoopStatement,
        cond_expr: &Expression,
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

        // Identify variables that are mutated in the loop body
        let mut mutated_vars = std::collections::HashSet::new();
        self.find_mutated_in_statements(&statement.body.statements, &mut mutated_vars);

        // Save the initial values and types of mutated variables
        // We'll create PHI nodes for these
        let mut phi_variables: Vec<(String, BasicValueEnum<'ctx>, ValueType, bool)> = Vec::new();

        for var_name in &mutated_vars {
            if let Some(var) = locals.get(var_name) {
                // Only create PHI for mutable variables that have a current value or pointer
                if var.mutable {
                    let initial_value = if let Some(ssa_value) = var.value {
                        // Already an SSA value
                        ssa_value
                    } else if let Some(ptr) = var.pointer {
                        // Load the current value from memory
                        self.load_from_pointer(ptr, &var.ty, var_name)?
                            .into_basic_value()
                            .ok_or_else(|| anyhow!("Cannot create PHI for void type"))?
                    } else {
                        continue; // Skip variables without values
                    };

                    phi_variables.push((
                        var_name.clone(),
                        initial_value,
                        var.ty.clone(),
                        true, // was_pointer flag
                    ));
                }
            }
        }

        // Branch to condition block
        if current_block.get_terminator().is_none() {
            map_builder_error(self.builder.build_unconditional_branch(cond_block))?;
        }

        self.builder.position_at_end(cond_block);

        // Create PHI nodes for all mutated variables
        let mut phi_nodes: HashMap<String, (inkwell::values::PhiValue<'ctx>, ValueType)> =
            HashMap::new();

        for (var_name, initial_value, ty, _) in &phi_variables {
            let phi = map_builder_error(
                self.builder
                    .build_phi(initial_value.get_type(), &format!("{}.phi", var_name)),
            )?;
            phi.add_incoming(&[(initial_value, current_block)]);
            phi_nodes.insert(var_name.clone(), (phi, ty.clone()));

            // Replace the variable in locals with the PHI value
            locals.insert(
                var_name.clone(),
                LocalVariable {
                    pointer: None,
                    value: Some(phi.as_basic_value()),
                    ty: ty.clone(),
                    mutable: true,
                },
            );
        }

        // Compile the loop condition
        let cond_value = self
            .compile_expression(cond_expr, function, locals)?
            .into_bool()?;

        map_builder_error(
            self.builder
                .build_conditional_branch(cond_value, body_block, exit_block),
        )?;

        self.builder.position_at_end(body_block);

        // Set loop context for break/continue (continue goes to condition for while loops)
        let old_loop_context = self.loop_context.take();
        self.loop_context = Some(LoopContext {
            continue_block: cond_block,
            exit_block,
        });

        // Create a map to track the "next" values for PHI variables
        // Start with the PHI values themselves
        let mut phi_next_values: HashMap<String, BasicValueEnum<'ctx>> = phi_nodes
            .iter()
            .map(|(name, (phi, _))| (name.clone(), phi.as_basic_value()))
            .collect();

        // Compile each statement in the loop body
        // We need to track assignments to PHI variables manually
        for (idx, stmt) in statement.body.statements.iter().enumerate() {
            // Before compiling, temporarily update locals to have latest PHI values
            for (var_name, next_value) in &phi_next_values {
                if let Some(var) = locals.get_mut(var_name) {
                    var.value = Some(*next_value);
                }
            }

            // Compile the statement
            let is_last = idx == statement.body.statements.len() - 1;
            let terminated =
                self.compile_statement(stmt, function, locals, return_type, is_last)?;

            // After compiling, check if any PHI variables were assigned
            for (var_name, (_phi, _ty)) in &phi_nodes {
                if let Some(var) = locals.get(var_name) {
                    if let Some(new_value) = var.value {
                        // Update the "next" value for this PHI variable
                        phi_next_values.insert(var_name.clone(), new_value);
                    }
                }
            }

            if terminated {
                break;
            }
        }

        // Restore loop context
        self.loop_context = old_loop_context;

        let body_terminated = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing block"))?
            .get_terminator()
            .is_some();

        if !body_terminated {
            let body_end_block = self
                .builder
                .get_insert_block()
                .ok_or_else(|| anyhow!("missing loop body end block"))?;

            // Add the incoming values from the loop body to the PHI nodes
            for (var_name, (phi, _ty)) in &phi_nodes {
                let phi_fallback = phi.as_basic_value();
                let updated_value = phi_next_values.get(var_name).unwrap_or(&phi_fallback);
                phi.add_incoming(&[(updated_value, body_end_block)]);
            }

            let back_edge = map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

            // Add loop metadata for optimization hints
            self.add_loop_metadata(back_edge);
        }

        self.builder.position_at_end(exit_block);

        // Restore pointer-based variables after loop (for variables that had pointers)
        // This allows assignments outside the loop to work correctly
        // Use the PHI nodes themselves (not the computed values from the loop body)
        for (var_name, _, ty, was_pointer) in &phi_variables {
            if *was_pointer {
                if let Some((phi, _)) = phi_nodes.get(var_name) {
                    // Allocate stack space and store the final PHI value
                    let alloca =
                        self.create_entry_alloca(function, var_name, self.basic_type(ty)?)?;
                    map_builder_error(self.builder.build_store(alloca, phi.as_basic_value()))?;

                    locals.insert(
                        var_name.clone(),
                        LocalVariable {
                            pointer: Some(alloca),
                            value: None,
                            ty: ty.clone(),
                            mutable: true,
                        },
                    );
                }
            }
        }

        Ok(false)
    }

    fn compile_for_loop(
        &mut self,
        statement: &LoopStatement,
        pattern: &ForPattern,
        iterator: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        return_type: &ValueType,
    ) -> Result<bool> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;

        // Create blocks for for-loop structure
        let cond_block = self.context.append_basic_block(function, "for_cond");
        let body_block = self.context.append_basic_block(function, "for_body");
        let inc_block = self.context.append_basic_block(function, "for_inc");
        let exit_block = self.context.append_basic_block(function, "for_exit");

        // Compile the iterator expression to get the collection
        let iterator_value = self.compile_expression(iterator, function, locals)?;

        // Get the list pointer and element type, or dict info
        let (list_ptr, element_type, is_dict, dict_ptr, value_type) = match &iterator_value {
            ExprValue::List {
                pointer,
                element_type,
            } => (*pointer, *element_type.clone(), false, None, None),
            ExprValue::Dict {
                pointer,
                value_type,
            } => {
                // For dict iteration, get the keys list
                let dict_keys_fn = self.ensure_dict_keys_fn();
                let keys_ptr = map_builder_error(self.builder.build_call(
                    dict_keys_fn,
                    &[(*pointer).into()],
                    "dict_keys",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected pointer from dict_keys"))?
                .into_pointer_value();
                (
                    keys_ptr,
                    ValueType::String,
                    true,
                    Some(*pointer),
                    Some(*value_type.clone()),
                )
            }
            _ => bail!("for loop iterator must be a List or Dict"),
        };

        // Get length of the collection
        let list_len_fn = self.ensure_list_len_ffi_fn();
        let length = map_builder_error(self.builder.build_call(
            list_len_fn,
            &[list_ptr.into()],
            "list_len",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected i64 from list_len"))?
        .into_int_value();

        // Identify variables that are mutated in the loop body
        let mut mutated_vars = std::collections::HashSet::new();
        self.find_mutated_in_statements(&statement.body.statements, &mut mutated_vars);

        // For for-loops, convert SSA variables to alloca-based storage to handle
        // continue correctly. When continue is called, it jumps to the increment
        // block, and we need the current value of mutated variables to be available
        // regardless of where continue was called from. Using allocas avoids PHI
        // complexity with multiple paths to the increment block.
        let mut converted_vars: Vec<(String, ValueType)> = Vec::new();
        for var_name in &mutated_vars {
            if let Some(var) = locals.get(var_name) {
                if var.mutable && var.pointer.is_none() {
                    // Variable uses SSA value, convert to alloca
                    if let Some(ssa_value) = var.value {
                        let alloca = self.create_entry_alloca(
                            function,
                            &format!("{}_for_loop", var_name),
                            self.basic_type(&var.ty)?,
                        )?;
                        map_builder_error(self.builder.build_store(alloca, ssa_value))?;
                        converted_vars.push((var_name.clone(), var.ty.clone()));
                        locals.insert(
                            var_name.clone(),
                            LocalVariable {
                                pointer: Some(alloca),
                                value: None,
                                ty: var.ty.clone(),
                                mutable: true,
                            },
                        );
                    }
                }
            }
        }

        // Branch to condition block
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // === COND BLOCK ===
        self.builder.position_at_end(cond_block);

        // Create index PHI node (only need PHI for the index, not for mutated vars)
        let i64_type = self.context.i64_type();
        let index_phi = map_builder_error(self.builder.build_phi(i64_type, "index.phi"))?;
        index_phi.add_incoming(&[(&i64_type.const_zero(), current_block)]);

        // Condition: index < length
        let cond = map_builder_error(self.builder.build_int_compare(
            inkwell::IntPredicate::SLT,
            index_phi.as_basic_value().into_int_value(),
            length,
            "loop_cond",
        ))?;

        map_builder_error(
            self.builder
                .build_conditional_branch(cond, body_block, exit_block),
        )?;

        // === BODY BLOCK ===
        self.builder.position_at_end(body_block);

        // Get the element at current index using inline LLVM code (matches compile_index)
        // TeaList: { tag: i8, len: i8, padding: [6 x i8], data: [8 x TeaValue] }
        let list_type = self
            .context
            .get_struct_type("TeaList")
            .ok_or_else(|| anyhow!("TeaList type not found"))?;
        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;
        // Get pointer to data array (field 3)
        let data_ptr = map_builder_error(
            self.builder
                .build_struct_gep(list_type, list_ptr, 3, "data_ptr"),
        )?;
        // Index into the data array
        let elem_ptr = unsafe {
            map_builder_error(self.builder.build_in_bounds_gep(
                tea_value_type.array_type(8),
                data_ptr,
                &[
                    self.context.i64_type().const_zero(),
                    index_phi.as_basic_value().into_int_value(),
                ],
                "elem_ptr",
            ))?
        };
        // Load the TeaValue
        let tea_value = map_builder_error(self.builder.build_load(
            tea_value_type,
            elem_ptr,
            "list_elem",
        ))?
        .into_struct_value();

        // Bind loop variable(s) based on pattern
        match pattern {
            ForPattern::Single(ident) => {
                // Convert TeaValue to the element type using the existing function
                let element_value = self.tea_value_to_expr(tea_value, element_type.clone())?;
                let basic_value = element_value
                    .into_basic_value()
                    .ok_or_else(|| anyhow!("loop variable cannot be void"))?;
                locals.insert(
                    ident.name.clone(),
                    LocalVariable {
                        pointer: None,
                        value: Some(basic_value),
                        ty: element_type.clone(),
                        mutable: false,
                    },
                );
            }
            ForPattern::Pair(key_ident, value_ident) => {
                if !is_dict {
                    bail!("for loop with two variables requires a Dict iterator");
                }
                let dict_ptr = dict_ptr.ok_or_else(|| anyhow!("missing dict pointer"))?;
                let value_type = value_type.ok_or_else(|| anyhow!("missing dict value type"))?;

                // The key is the string we got from the keys list - use tea_value_to_expr
                let key_value = self.tea_value_to_expr(tea_value, ValueType::String)?;
                let key_ptr = key_value.into_string()?;

                // Get the value from dict using this key
                let dict_get_fn = self.ensure_dict_get();
                let dict_value = map_builder_error(self.builder.build_call(
                    dict_get_fn,
                    &[dict_ptr.into(), key_ptr.into()],
                    "dict_get",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue from dict_get"))?
                .into_struct_value();

                let value_expr = self.tea_value_to_expr(dict_value, value_type.clone())?;
                let value_basic = value_expr
                    .into_basic_value()
                    .ok_or_else(|| anyhow!("dict value cannot be void"))?;

                // Bind key (String)
                locals.insert(
                    key_ident.name.clone(),
                    LocalVariable {
                        pointer: None,
                        value: Some(key_ptr.into()),
                        ty: ValueType::String,
                        mutable: false,
                    },
                );

                // Bind value
                locals.insert(
                    value_ident.name.clone(),
                    LocalVariable {
                        pointer: None,
                        value: Some(value_basic),
                        ty: value_type,
                        mutable: false,
                    },
                );
            }
        }

        // Set loop context for break/continue (continue goes to increment for for-loops)
        let old_loop_context = self.loop_context.take();
        self.loop_context = Some(LoopContext {
            continue_block: inc_block,
            exit_block,
        });

        // Compile loop body
        // Since mutated variables now use allocas, no PHI tracking is needed
        for (idx, stmt) in statement.body.statements.iter().enumerate() {
            let is_last = idx == statement.body.statements.len() - 1;
            let terminated =
                self.compile_statement(stmt, function, locals, return_type, is_last)?;

            if terminated {
                break;
            }
        }

        // Restore loop context
        self.loop_context = old_loop_context;

        // Remove loop variables from locals
        match pattern {
            ForPattern::Single(ident) => {
                locals.remove(&ident.name);
            }
            ForPattern::Pair(key_ident, value_ident) => {
                locals.remove(&key_ident.name);
                locals.remove(&value_ident.name);
            }
        }

        let body_terminated = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing block"))?
            .get_terminator()
            .is_some();

        if !body_terminated {
            // Body is not terminated, so branch to increment block
            map_builder_error(self.builder.build_unconditional_branch(inc_block))?;
        }

        // === INCREMENT BLOCK ===
        self.builder.position_at_end(inc_block);

        // index.next = index + 1
        let index_next = map_builder_error(self.builder.build_int_add(
            index_phi.as_basic_value().into_int_value(),
            i64_type.const_int(1, false),
            "index.next",
        ))?;

        // Add incoming to index PHI from increment block
        index_phi.add_incoming(&[(&index_next, inc_block)]);

        let back_edge = map_builder_error(self.builder.build_unconditional_branch(cond_block))?;
        self.add_loop_metadata(back_edge);

        // === EXIT BLOCK ===
        self.builder.position_at_end(exit_block);

        // Variables that were converted to allocas for the loop remain as allocas
        // (they're already in locals with pointer set)

        Ok(false)
    }

    /// Ensure the tea_list_len_ffi FFI function is declared
    fn ensure_list_len_ffi_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.list_len_ffi_fn {
            return func;
        }
        let fn_type = self
            .context
            .i64_type()
            .fn_type(&[self.list_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_list_len_ffi", fn_type, Some(Linkage::External));
        self.list_len_ffi_fn = Some(func);
        func
    }

    /// Ensure the tea_dict_keys FFI function is declared
    fn ensure_dict_keys_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_keys_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.dict_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_dict_keys", fn_type, Some(Linkage::External));
        self.dict_keys_fn = Some(func);
        func
    }

    /// Ensure the tea_dict_values FFI function is declared
    fn ensure_dict_values_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_values_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.dict_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_dict_values", fn_type, Some(Linkage::External));
        self.dict_values_fn = Some(func);
        func
    }

    /// Ensure the tea_dict_entries FFI function is declared
    fn ensure_dict_entries_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.dict_entries_fn {
            return func;
        }
        let fn_type = self
            .list_ptr_type()
            .fn_type(&[self.dict_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_dict_entries", fn_type, Some(Linkage::External));
        self.dict_entries_fn = Some(func);
        func
    }

    /// Convert a TeaValue struct to an ExprValue based on expected type
    fn tea_value_to_expr_value(
        &mut self,
        tea_value: inkwell::values::StructValue<'ctx>,
        expected_type: &ValueType,
    ) -> Result<ExprValue<'ctx>> {
        // Allocate the TeaValue on the stack and get a pointer to it
        // (the tea_value_as_* functions expect a pointer due to ARM64 ABI issues)
        let value_alloca = map_builder_error(
            self.builder
                .build_alloca(self.value_type(), "tea_value_temp"),
        )?;
        map_builder_error(self.builder.build_store(value_alloca, tea_value))?;

        match expected_type {
            ValueType::Int => {
                let value_as_int = self.ensure_value_as_int();
                let int_val = map_builder_error(self.builder.build_call(
                    value_as_int,
                    &[value_alloca.into()],
                    "as_int",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected i64"))?
                .into_int_value();
                Ok(ExprValue::Int(int_val))
            }
            ValueType::Float => {
                let value_as_float = self.ensure_value_as_float();
                let float_val = map_builder_error(self.builder.build_call(
                    value_as_float,
                    &[value_alloca.into()],
                    "as_float",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected f64"))?
                .into_float_value();
                Ok(ExprValue::Float(float_val))
            }
            ValueType::Bool => {
                let value_as_bool = self.ensure_value_as_bool();
                let bool_val = map_builder_error(self.builder.build_call(
                    value_as_bool,
                    &[value_alloca.into()],
                    "as_bool",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected i32"))?
                .into_int_value();
                Ok(ExprValue::Bool(bool_val))
            }
            ValueType::String => {
                let value_as_string = self.ensure_value_as_string();
                let str_ptr = map_builder_error(self.builder.build_call(
                    value_as_string,
                    &[value_alloca.into()],
                    "as_string",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected ptr"))?
                .into_pointer_value();
                Ok(ExprValue::String(str_ptr))
            }
            ValueType::List(element_type) => {
                let value_as_list = self.ensure_value_as_list();
                let list_ptr = map_builder_error(self.builder.build_call(
                    value_as_list,
                    &[value_alloca.into()],
                    "as_list",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected ptr"))?
                .into_pointer_value();
                Ok(ExprValue::List {
                    pointer: list_ptr,
                    element_type: element_type.clone(),
                })
            }
            ValueType::Dict(value_type) => {
                let value_as_dict = self.ensure_value_as_dict();
                let dict_ptr = map_builder_error(self.builder.build_call(
                    value_as_dict,
                    &[value_alloca.into()],
                    "as_dict",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected ptr"))?
                .into_pointer_value();
                Ok(ExprValue::Dict {
                    pointer: dict_ptr,
                    value_type: value_type.clone(),
                })
            }
            _ => bail!("unsupported element type for for-loop: {:?}", expected_type),
        }
    }

    fn compile_assignment(
        &mut self,
        assignment: &crate::ast::AssignmentExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match &assignment.target.kind {
            ExpressionKind::Identifier(identifier) => {
                if let Some(variable) = locals.get(identifier.name.as_str()).cloned() {
                    if !variable.mutable {
                        bail!(format!(
                            "cannot assign to const '{}' at {}",
                            identifier.name,
                            Self::describe_span(assignment.target.span)
                        ));
                    }

                    let var_ty = variable.ty.clone();

                    // Optimization: detect `var = var + expr` pattern for strings
                    // and use efficient push_str instead of concat
                    if let ValueType::String = &var_ty {
                        if let Some(optimized) = self.try_compile_string_push_assignment(
                            &identifier.name,
                            &assignment.value,
                            function,
                            locals,
                            &variable,
                        )? {
                            return Ok(optimized);
                        }
                    }

                    let value = self.compile_expression(&assignment.value, function, locals)?;
                    let converted_value = self.convert_expr_to_type(value, &var_ty)?;

                    // Check if this is an SSA value (PHI node in loop) or pointer
                    if let Some(pointer) = variable.pointer {
                        // Traditional pointer-based assignment
                        self.store_expr_in_pointer(
                            pointer,
                            &var_ty,
                            converted_value,
                            &identifier.name,
                        )?;
                    } else if variable.value.is_some() && variable.mutable {
                        // SSA value in loop (PHI node) - update the local with the new value
                        let new_basic_value = converted_value
                            .into_basic_value()
                            .ok_or_else(|| anyhow!("Cannot assign void value"))?;

                        locals.insert(
                            identifier.name.clone(),
                            LocalVariable {
                                pointer: None,
                                value: Some(new_basic_value),
                                ty: var_ty,
                                mutable: true,
                            },
                        );
                    } else {
                        bail!("Cannot assign to immutable parameter '{}'", identifier.name);
                    }

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
            ExpressionKind::Index(index_expr) => {
                // Handle indexed assignment: dict[key] = value or list[index] = value
                if let ExpressionKind::Identifier(identifier) = &index_expr.object.kind {
                    self.compile_indexed_assignment(
                        &identifier.name,
                        &index_expr.index,
                        &assignment.value,
                        function,
                        locals,
                    )
                } else {
                    bail!("indexed assignment only supports simple identifiers as base")
                }
            }
            _ => bail!("only identifier and indexed assignment supported"),
        }
    }

    /// Try to optimize `var = var + expr` into a push_str operation for strings.
    /// Returns Some(ExprValue::Void) if the optimization was applied, None otherwise.
    fn try_compile_string_push_assignment(
        &mut self,
        target_name: &str,
        value_expr: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        variable: &LocalVariable<'ctx>,
    ) -> Result<Option<ExprValue<'ctx>>> {
        // Check if value is a binary Add expression where the left side is the target variable
        if let ExpressionKind::Binary(binary) = &value_expr.kind {
            if matches!(binary.operator, BinaryOperator::Add) {
                if let ExpressionKind::Identifier(left_id) = &binary.left.kind {
                    if left_id.name == target_name {
                        // This is `var = var + expr` - use push_str optimization
                        // Compile the right-hand side (the part being appended)
                        let rhs = self.compile_expression(&binary.right, function, locals)?;
                        let rhs_ptr = rhs.into_string()?;

                        // Load current string value
                        let current_ptr = if let Some(pointer) = variable.pointer {
                            let loaded =
                                self.load_from_pointer(pointer, &ValueType::String, "current_str")?;
                            loaded.into_string()?
                        } else if let Some(value) = variable.value {
                            value.into_pointer_value()
                        } else {
                            return Ok(None);
                        };

                        // Call push_str which mutates in place
                        let new_ptr = self.push_string_value(current_ptr, rhs_ptr)?;

                        // Store the (possibly reallocated) result back
                        if let Some(pointer) = variable.pointer {
                            map_builder_error(self.builder.build_store(pointer, new_ptr))?;
                        } else if variable.value.is_some() && variable.mutable {
                            // SSA value - update local
                            locals.insert(
                                target_name.to_string(),
                                LocalVariable {
                                    pointer: None,
                                    value: Some(new_ptr.into()),
                                    ty: ValueType::String,
                                    mutable: true,
                                },
                            );
                        }

                        return Ok(Some(ExprValue::Void));
                    }
                }
            }
        }
        Ok(None)
    }

    fn compile_indexed_assignment(
        &mut self,
        base_name: &str,
        index: &Expression,
        value: &Expression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Load the current collection (dict or list)
        let (collection_ptr, collection_ty, is_local, _is_mutable) =
            if let Some(variable) = locals.get(base_name) {
                if !variable.mutable {
                    bail!("cannot mutate const '{}'", base_name);
                }
                let pointer = variable.pointer.ok_or_else(|| {
                    anyhow!("Cannot assign to immutable parameter '{}'", base_name)
                })?;
                (pointer, variable.ty.clone(), true, true)
            } else if let Some(slot) = self.global_slots.get(base_name).cloned() {
                if !slot.mutable {
                    bail!("cannot mutate const '{}'", base_name);
                }
                (
                    slot.pointer.as_pointer_value(),
                    slot.ty.clone(),
                    false,
                    true,
                )
            } else {
                bail!("undefined variable '{}'", base_name);
            };

        // Load the collection from memory
        let loaded = self.load_from_pointer(collection_ptr, &collection_ty, base_name)?;

        match loaded {
            ExprValue::Dict {
                pointer,
                value_type: _,
            } => {
                // Compile the key expression
                let key_expr = self.compile_expression(index, function, locals)?;
                let key_ptr = match key_expr {
                    ExprValue::String(ptr) => ptr,
                    _ => bail!("dictionary index expects a String key"),
                };

                // Compile the new value
                let new_value = self.compile_expression(value, function, locals)?;

                // Call tea_dict_set to mutate the dictionary in place
                let dict_set = self.ensure_dict_set();
                let tea_value = self.expr_to_tea_value(new_value)?;
                self.call_function(
                    dict_set,
                    &[pointer.into(), key_ptr.into(), tea_value.into()],
                    "dict_set",
                )?;

                Ok(ExprValue::Void)
            }
            ExprValue::List {
                pointer,
                element_type,
            } => {
                // Compile the index expression
                let index_expr = self.compile_expression(index, function, locals)?;
                let index_value = index_expr.into_int()?;

                // Compile the new value
                let new_value = self.compile_expression(value, function, locals)?;

                // Call list_set which returns a new list
                let list_set = self.ensure_list_set();
                let tea_value = self.expr_to_tea_value(new_value)?;
                let new_list_ptr = self
                    .call_function(
                        list_set,
                        &[pointer.into(), index_value.into(), tea_value.into()],
                        "list_set",
                    )?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("expected list pointer"))?
                    .into_pointer_value();

                // Store the new list back
                let new_list = ExprValue::List {
                    pointer: new_list_ptr,
                    element_type,
                };
                self.store_expr_in_pointer(collection_ptr, &collection_ty, new_list, base_name)?;

                if !is_local {
                    if let Some(slot_mut) = self.global_slots.get_mut(base_name) {
                        slot_mut.initialized = true;
                    }
                }

                Ok(ExprValue::Void)
            }
            _ => bail!("indexed assignment requires a list or dictionary"),
        }
    }

    fn compile_list_literal(
        &mut self,
        list: &crate::ast::ListLiteral,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Route to inline or heap based on element count
        if list.elements.len() < 8 {
            self.compile_small_list_literal(list, function, locals)
        } else {
            self.compile_heap_list_literal(list, function, locals)
        }
    }

    fn compile_small_list_literal(
        &mut self,
        list: &crate::ast::ListLiteral,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        let len = list.elements.len();

        // Get TeaList struct type from the context
        let list_type = self
            .context
            .get_struct_type("TeaList")
            .ok_or_else(|| anyhow!("TeaList type not found"))?;

        // Allocate SmallList on heap using malloc (136 bytes)
        let malloc_fn = self.ensure_malloc_fn();
        let size = self.context.i64_type().const_int(136, false); // sizeof(TeaList)
        let call = self.call_function(malloc_fn, &[size.into()], "malloc_small_list")?;
        let list_ptr = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("malloc returned no value"))?
            .into_pointer_value();

        // Set tag = 1 (inline)
        let tag_ptr = map_builder_error(
            self.builder
                .build_struct_gep(list_type, list_ptr, 0, "tag_ptr"),
        )?;
        map_builder_error(
            self.builder
                .build_store(tag_ptr, self.context.i8_type().const_int(1, false)),
        )?;

        // Set length
        let len_ptr = map_builder_error(
            self.builder
                .build_struct_gep(list_type, list_ptr, 1, "len_ptr"),
        )?;
        map_builder_error(
            self.builder
                .build_store(len_ptr, self.context.i8_type().const_int(len as u64, false)),
        )?;

        // Get pointer to data array (field index 3)
        let data_ptr = map_builder_error(
            self.builder
                .build_struct_gep(list_type, list_ptr, 3, "data_ptr"),
        )?;

        // Compile and store each element
        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;
        let mut element_type: Option<ValueType> = None;

        for (i, element) in list.elements.iter().enumerate() {
            let expr = self.compile_expression(element, function, locals)?;
            let expr_type = expr.ty();
            if let Some(existing) = &element_type {
                if *existing != expr_type {
                    bail!("list literal elements must share a type");
                }
            } else {
                element_type = Some(expr_type.clone());
            }

            let tea_value = self.expr_to_tea_value(expr)?;

            // Get pointer to array element
            let elem_ptr = unsafe {
                map_builder_error(self.builder.build_in_bounds_gep(
                    tea_value_type.array_type(8),
                    data_ptr,
                    &[
                        self.context.i32_type().const_zero(),
                        self.context.i32_type().const_int(i as u64, false),
                    ],
                    &format!("elem_{}", i),
                ))?
            };

            // Store the TeaValue
            map_builder_error(self.builder.build_store(elem_ptr, tea_value))?;
        }

        let element_type = element_type.unwrap_or(ValueType::Void);
        Ok(ExprValue::List {
            pointer: list_ptr,
            element_type: Box::new(element_type),
        })
    }

    fn compile_heap_list_literal(
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

        // Check if this is a slice operation (range index)
        if let ExpressionKind::Range(range) = &index.index.kind {
            let start_expr = self.compile_expression(&range.start, function, locals)?;
            let end_expr = self.compile_expression(&range.end, function, locals)?;
            let start_value = start_expr.into_int()?;
            let end_value = end_expr.into_int()?;
            let inclusive_value = self
                .context
                .bool_type()
                .const_int(if range.inclusive { 1 } else { 0 }, false);

            match object {
                ExprValue::String(string_ptr) => {
                    let slice_fn = self.ensure_string_slice();
                    let result_ptr = self
                        .call_function(
                            slice_fn,
                            &[
                                string_ptr.into(),
                                start_value.into(),
                                end_value.into(),
                                inclusive_value.into(),
                            ],
                            "string_slice",
                        )?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("expected String from string_slice"))?
                        .into_pointer_value();
                    Ok(ExprValue::String(result_ptr))
                }
                ExprValue::List {
                    pointer,
                    element_type,
                } => {
                    let slice_fn = self.ensure_list_slice();
                    let result_ptr = self
                        .call_function(
                            slice_fn,
                            &[
                                pointer.into(),
                                start_value.into(),
                                end_value.into(),
                                inclusive_value.into(),
                            ],
                            "list_slice",
                        )?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("expected List from list_slice"))?
                        .into_pointer_value();
                    Ok(ExprValue::List {
                        pointer: result_ptr,
                        element_type,
                    })
                }
                _ => bail!("slicing expects a list or string value"),
            }
        } else {
            // Regular indexing
            let key_expr = self.compile_expression(&index.index, function, locals)?;
            match object {
                ExprValue::List {
                    pointer,
                    element_type,
                } => {
                    let index_value = key_expr.into_int()?;
                    // Inline list access for small lists
                    // TeaList: { tag: i8, len: i8, padding: [6 x i8], data: [8 x TeaValue] }
                    let list_type = self
                        .context
                        .get_struct_type("TeaList")
                        .ok_or_else(|| anyhow!("TeaList type not found"))?;
                    let tea_value_type = self
                        .context
                        .get_struct_type("TeaValue")
                        .ok_or_else(|| anyhow!("TeaValue type not found"))?;
                    // Get pointer to data array (field 3)
                    let data_ptr = map_builder_error(
                        self.builder
                            .build_struct_gep(list_type, pointer, 3, "data_ptr"),
                    )?;
                    // Index into the data array
                    let elem_ptr = unsafe {
                        map_builder_error(self.builder.build_in_bounds_gep(
                            tea_value_type.array_type(8),
                            data_ptr,
                            &[self.context.i64_type().const_zero(), index_value],
                            "elem_ptr",
                        ))?
                    };
                    // Load the TeaValue
                    let tea_value = map_builder_error(self.builder.build_load(
                        tea_value_type,
                        elem_ptr,
                        "list_elem",
                    ))?
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
                ExprValue::String(string_ptr) => {
                    let index_value = key_expr.into_int()?;
                    let string_index_fn = self.ensure_string_index();
                    let result_ptr = self
                        .call_function(
                            string_index_fn,
                            &[string_ptr.into(), index_value.into()],
                            "string_index",
                        )?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("expected String from string_index"))?
                        .into_pointer_value();
                    Ok(ExprValue::String(result_ptr))
                }
                _ => bail!("indexing expects a list, dict, or string value"),
            }
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

                // Inline struct field access to avoid ABI issues with TeaValue return
                // TeaStructInstance: { template: ptr, fields: ptr }
                // Load the fields pointer (field 1), then index into it
                let struct_instance_type = self
                    .context
                    .get_struct_type("TeaStructInstance")
                    .ok_or_else(|| anyhow!("TeaStructInstance type not found"))?;
                let tea_value_type = self
                    .context
                    .get_struct_type("TeaValue")
                    .ok_or_else(|| anyhow!("TeaValue type not found"))?;

                // Get pointer to fields field (index 1)
                let fields_ptr_ptr = map_builder_error(self.builder.build_struct_gep(
                    struct_instance_type,
                    pointer,
                    1,
                    "fields_ptr_ptr",
                ))?;

                // Load the fields pointer
                let fields_ptr = map_builder_error(self.builder.build_load(
                    self.ptr_type,
                    fields_ptr_ptr,
                    "fields_ptr",
                ))?
                .into_pointer_value();

                // Index into the fields array to get pointer to TeaValue
                let field_ptr = unsafe {
                    map_builder_error(self.builder.build_in_bounds_gep(
                        tea_value_type,
                        fields_ptr,
                        &[self.context.i64_type().const_int(index as u64, false)],
                        "field_ptr",
                    ))?
                };

                // Get pointer to payload field within TeaValue (field 2 is payload)
                // TeaValue: { tag: i32 (field 0), padding: i32 (field 1), payload: i64 (field 2) }
                let payload_ptr = map_builder_error(self.builder.build_struct_gep(
                    tea_value_type,
                    field_ptr,
                    2,
                    "field_payload_ptr",
                ))?;
                let payload = map_builder_error(self.builder.build_load(
                    self.context.i64_type(),
                    payload_ptr,
                    "field_payload",
                ))?
                .into_int_value();

                self.payload_to_expr(payload, field_type)
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
                // Use output pointer to avoid ARM64 ABI issues with TeaValue return
                let get_fn = self.ensure_error_get();
                let tea_value_type = self
                    .context
                    .get_struct_type("TeaValue")
                    .ok_or_else(|| anyhow!("TeaValue type not found"))?;
                let out_alloca = map_builder_error(
                    self.builder.build_alloca(tea_value_type, "error_field_out"),
                )?;
                self.call_function(
                    get_fn,
                    &[
                        pointer.into(),
                        self.int_type().const_int(index as u64, false).into(),
                        out_alloca.into(),
                    ],
                    "error_get_field",
                )?;
                let tea_value = map_builder_error(self.builder.build_load(
                    tea_value_type,
                    out_alloca,
                    "error_field_value",
                ))?
                .into_struct_value();
                self.tea_value_to_expr(tea_value, field_type)
            }
            _ => bail!("member access expects a struct value"),
        }
    }

    fn compile_conditional_expression(
        &mut self,
        expression: &ConditionalExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Compile condition
        let condition = self
            .compile_expression(&expression.condition, function, locals)?
            .into_bool()?;

        // Compile then branch first to determine result type
        let then_block = self.context.append_basic_block(function, "if_expr_then");
        let else_block = self.context.append_basic_block(function, "if_expr_else");
        let merge_block = self.context.append_basic_block(function, "if_expr_merge");

        // Branch based on condition
        map_builder_error(
            self.builder
                .build_conditional_branch(condition, then_block, else_block),
        )?;

        // Compile then branch
        self.builder.position_at_end(then_block);
        let then_value = self.compile_expression(&expression.consequent, function, locals)?;
        let result_type = then_value.ty();

        // Allocate storage for result
        let needs_storage = !matches!(result_type, ValueType::Void);
        let result_alloca = if needs_storage {
            Some(self.create_entry_alloca(
                function,
                "if_expr_result",
                self.basic_type(&result_type)?,
            )?)
        } else {
            None
        };

        // Store then branch result
        if let Some(alloca) = result_alloca {
            self.store_expr_in_pointer(alloca, &result_type, then_value, "if_expr_then")?;
        }
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;

        // Compile else branch
        self.builder.position_at_end(else_block);
        let else_value = self.compile_expression(&expression.alternative, function, locals)?;
        if let Some(alloca) = result_alloca {
            self.store_expr_in_pointer(alloca, &result_type, else_value, "if_expr_else")?;
        }
        map_builder_error(self.builder.build_unconditional_branch(merge_block))?;

        // Load result in merge block
        self.builder.position_at_end(merge_block);
        if let Some(alloca) = result_alloca {
            self.load_from_pointer(alloca, &result_type, "if_expr_result")
        } else {
            Ok(ExprValue::Void)
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
                            pointer: Some(alloca),
                            value: None,
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
                                pointer: Some(binding_alloca),
                                value: None,
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
                pointer: Some(alloca),
                value: None,
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
        self.push_function_can_throw(false); // lambdas don't throw by default
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
                    pointer: Some(alloca),
                    value: None,
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
                    pointer: Some(alloca),
                    value: None,
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
        self.pop_function_can_throw();
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
        let len = bytes.len();

        // Small string optimization: inline strings < 23 bytes
        if len < 23 {
            return self.compile_small_string_literal(bytes);
        }

        // Large string: heap allocation with tag=0
        self.compile_heap_string_literal(bytes)
    }

    fn compile_small_string_literal(&mut self, bytes: &[u8]) -> Result<ExprValue<'ctx>> {
        let len = bytes.len();

        // Get TeaString struct type from the context
        let string_type = self
            .context
            .get_struct_type("TeaString")
            .ok_or_else(|| anyhow!("TeaString type not found"))?;

        // Allocate SmallString on heap using malloc (24 bytes)
        let malloc_fn = self.ensure_malloc_fn();
        let size = self.context.i64_type().const_int(24, false); // sizeof(TeaString)
        let call = self.call_function(malloc_fn, &[size.into()], "malloc_small_str")?;
        let string_ptr = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("malloc returned no value"))?
            .into_pointer_value();

        // Set tag = 1 (inline)
        let tag_ptr = map_builder_error(self.builder.build_struct_gep(
            string_type,
            string_ptr,
            0,
            "tag_ptr",
        ))?;
        map_builder_error(
            self.builder
                .build_store(tag_ptr, self.context.i8_type().const_int(1, false)),
        )?;

        // Set length
        let len_ptr = map_builder_error(self.builder.build_struct_gep(
            string_type,
            string_ptr,
            1,
            "len_ptr",
        ))?;
        map_builder_error(
            self.builder
                .build_store(len_ptr, self.context.i8_type().const_int(len as u64, false)),
        )?;

        // Copy bytes into inline data array
        let data_ptr = map_builder_error(self.builder.build_struct_gep(
            string_type,
            string_ptr,
            2,
            "data_ptr",
        ))?;

        for (i, &byte) in bytes.iter().enumerate() {
            let elem_ptr = unsafe {
                map_builder_error(self.builder.build_in_bounds_gep(
                    self.context.i8_type().array_type(22),
                    data_ptr,
                    &[
                        self.context.i32_type().const_zero(),
                        self.context.i32_type().const_int(i as u64, false),
                    ],
                    &format!("byte_{}", i),
                ))?
            };
            map_builder_error(self.builder.build_store(
                elem_ptr,
                self.context.i8_type().const_int(byte as u64, false),
            ))?;
        }

        Ok(ExprValue::String(string_ptr))
    }

    fn compile_heap_string_literal(&mut self, bytes: &[u8]) -> Result<ExprValue<'ctx>> {
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
            ExpressionKind::Conditional(cond) => {
                self.compile_conditional_expression(cond, function, locals)
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
        // If variable has a direct SSA value (immutable parameter), use it
        if let Some(value) = variable.value {
            return self.basic_value_to_expr_value(value, &variable.ty);
        }

        // Otherwise load from pointer (mutable variable)
        let pointer = variable
            .pointer
            .ok_or_else(|| anyhow!("LocalVariable '{}' has neither value nor pointer", name))?;
        self.load_from_pointer(pointer, &variable.ty, name)
    }

    /// Convert a BasicValueEnum to an ExprValue based on the type
    fn basic_value_to_expr_value(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &ValueType,
    ) -> Result<ExprValue<'ctx>> {
        match ty {
            ValueType::Int => Ok(ExprValue::Int(value.into_int_value())),
            ValueType::Float => Ok(ExprValue::Float(value.into_float_value())),
            ValueType::Bool => Ok(ExprValue::Bool(value.into_int_value())),
            ValueType::String => Ok(ExprValue::String(value.into_pointer_value())),
            ValueType::List(inner) => Ok(ExprValue::List {
                pointer: value.into_pointer_value(),
                element_type: inner.clone(),
            }),
            ValueType::Dict(inner) => Ok(ExprValue::Dict {
                pointer: value.into_pointer_value(),
                value_type: inner.clone(),
            }),
            ValueType::Struct(struct_name) => Ok(ExprValue::Struct {
                pointer: value.into_pointer_value(),
                struct_name: struct_name.clone(),
            }),
            ValueType::Function(param_types, return_type) => Ok(ExprValue::Closure {
                pointer: value.into_pointer_value(),
                param_types: param_types.clone(),
                return_type: return_type.clone(),
            }),
            ValueType::Optional(inner) => Ok(ExprValue::Optional {
                value: value.into_struct_value(),
                inner: inner.clone(),
            }),
            ValueType::Error {
                error_name,
                variant_name,
            } => Ok(ExprValue::Error {
                pointer: value.into_pointer_value(),
                error_name: error_name.clone(),
                variant_name: variant_name.clone(),
            }),
            ValueType::Void => Ok(ExprValue::Void),
        }
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

    /// Compile a call expression with optional tail call hint
    fn compile_call_with_tail_hint(
        &mut self,
        call: &CallExpression,
        span: SourceSpan,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        is_tail_call: bool,
    ) -> Result<ExprValue<'ctx>> {
        self.compile_call_internal(call, span, function, locals, is_tail_call)
    }

    fn compile_call(
        &mut self,
        call: &CallExpression,
        span: SourceSpan,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        self.compile_call_internal(call, span, function, locals, false)
    }

    fn compile_call_internal(
        &mut self,
        call: &CallExpression,
        span: SourceSpan,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        is_tail_call: bool,
    ) -> Result<ExprValue<'ctx>> {
        if let ExpressionKind::Member(member) = &call.callee.kind {
            if let ExpressionKind::Identifier(alias_ident) = &member.object.kind {
                if let Some(functions) = self.module_builtins.get(&alias_ident.name) {
                    if let Some(&kind) = functions.get(&member.property) {
                        return self.compile_builtin_call(kind, call, function, locals);
                    }
                }
            }

            // Check for List/Dict method calls, but only if the object is not
            // an error type, struct, or module identifier (those are handled separately)
            let should_check_methods =
                if let ExpressionKind::Identifier(ident) = &member.object.kind {
                    // Skip method check if this identifier is an error type, struct, or module
                    !self.error_definitions_tc.contains_key(&ident.name)
                        && !self.structs.contains_key(&ident.name)
                        && !self.module_builtins.contains_key(&ident.name)
                } else {
                    true
                };

            if should_check_methods {
                // Check for List method calls (map, filter, reduce, find, any, all)
                let object = self.compile_expression(&member.object, function, locals)?;
                if let ExprValue::List {
                    pointer,
                    element_type,
                } = &object
                {
                    if matches!(
                        member.property.as_str(),
                        "map" | "filter" | "reduce" | "find" | "any" | "all"
                    ) {
                        return self.compile_list_method_call(
                            &member.property,
                            *pointer,
                            element_type.clone(),
                            call,
                            function,
                            locals,
                        );
                    }
                }

                // Check for Dict method calls (keys, values, entries)
                if let ExprValue::Dict {
                    pointer,
                    value_type,
                } = &object
                {
                    if matches!(member.property.as_str(), "keys" | "values" | "entries") {
                        return self.compile_dict_method_call(
                            &member.property,
                            *pointer,
                            value_type.clone(),
                            call,
                            function,
                            locals,
                        );
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

            // Check for intrinsic function calls
            if let Some(intrinsic) = Intrinsic::from_name(&identifier.name) {
                return self.compile_intrinsic_call(intrinsic, call, function, locals);
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

                    // Mark as tail call if requested
                    if is_tail_call {
                        call_site.set_tail_call(true);
                    }

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
            StdFunctionKind::Println => {
                self.compile_println_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ToString => {
                self.compile_to_string_call(&call.arguments, function, locals)
            }
            StdFunctionKind::TypeOf => self.compile_type_of_call(&call.arguments, function, locals),
            StdFunctionKind::Panic => self.compile_panic_call(&call.arguments, function, locals),
            StdFunctionKind::Exit => self.compile_exit_call(&call.arguments, function, locals),
            StdFunctionKind::Args => self.compile_args_call(),
            StdFunctionKind::ReadLine => self.compile_read_line_call(),
            StdFunctionKind::ReadAll => self.compile_read_all_call(),
            StdFunctionKind::Eprint => self.compile_eprint_call(&call.arguments, function, locals),
            StdFunctionKind::Eprintln => {
                self.compile_eprintln_call(&call.arguments, function, locals)
            }
            StdFunctionKind::IsTty => self.compile_is_tty_call(),
            StdFunctionKind::Length => self.compile_length_call(&call.arguments, function, locals),
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
            StdFunctionKind::UtilToString => {
                self.compile_util_to_string_call(&call.arguments, function, locals)
            }
            StdFunctionKind::StringIndexOf => {
                self.compile_string_index_of_call(&call.arguments, function, locals)
            }
            StdFunctionKind::StringSplit => {
                self.compile_string_split_call(&call.arguments, function, locals)
            }
            StdFunctionKind::StringContains => {
                self.compile_string_contains_call(&call.arguments, function, locals)
            }
            StdFunctionKind::StringReplace => {
                self.compile_string_replace_call(&call.arguments, function, locals)
            }
            StdFunctionKind::StringToLower => {
                self.compile_string_to_lower_call(&call.arguments, function, locals)
            }
            StdFunctionKind::StringToUpper => {
                self.compile_string_to_upper_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathFloor => {
                self.compile_math_floor_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathCeil => {
                self.compile_math_ceil_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathRound => {
                self.compile_math_round_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathAbs => {
                self.compile_math_abs_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathSqrt => {
                self.compile_math_sqrt_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathMin => {
                self.compile_math_min_call(&call.arguments, function, locals)
            }
            StdFunctionKind::MathMax => {
                self.compile_math_max_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvGet => self.compile_env_get_call(&call.arguments, function, locals),
            StdFunctionKind::EnvSet => self.compile_env_set_call(&call.arguments, function, locals),
            StdFunctionKind::EnvVars => {
                self.compile_env_vars_call(&call.arguments, function, locals)
            }
            StdFunctionKind::EnvCwd => self.compile_env_cwd_call(&call.arguments, function, locals),
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
            StdFunctionKind::FsReadText => {
                self.compile_fs_read_text_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsWriteText => {
                self.compile_fs_write_text_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsCreateDir => {
                self.compile_fs_create_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsRemove => {
                self.compile_fs_remove_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsListDir => {
                self.compile_fs_list_dir_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsRename => {
                self.compile_fs_rename_call(&call.arguments, function, locals)
            }
            StdFunctionKind::FsStat => self.compile_fs_stat_call(&call.arguments, function, locals),
            // Process execution
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
                self.compile_process_read_stdout_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessReadStderr => {
                self.compile_process_read_stderr_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessWriteStdin => {
                self.compile_process_write_stdin_call(&call.arguments, function, locals)
            }
            StdFunctionKind::ProcessCloseStdin => {
                self.compile_process_close_stdin_call(&call.arguments, function, locals)
            }
            // Args intrinsics
            StdFunctionKind::ArgsAll => self.compile_args_call(),
            StdFunctionKind::ArgsProgram => self.compile_args_program_call(),
            // Regex module
            StdFunctionKind::RegexCompile => {
                self.compile_regex_compile_call(&call.arguments, function, locals)
            }
            StdFunctionKind::RegexIsMatch => {
                self.compile_regex_is_match_call(&call.arguments, function, locals)
            }
            StdFunctionKind::RegexFindAll => {
                self.compile_regex_find_all_call(&call.arguments, function, locals)
            }
            StdFunctionKind::RegexCaptures => {
                self.compile_regex_captures_call(&call.arguments, function, locals)
            }
            StdFunctionKind::RegexReplace => {
                self.compile_regex_replace_call(&call.arguments, function, locals)
            }
            StdFunctionKind::RegexReplaceAll => {
                self.compile_regex_replace_all_call(&call.arguments, function, locals)
            }
            StdFunctionKind::RegexSplit => {
                self.compile_regex_split_call(&call.arguments, function, locals)
            }
        }
    }

    fn compile_intrinsic_call(
        &mut self,
        intrinsic: Intrinsic,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match intrinsic {
            // Type predicates (removed - now handled by type system)

            // Conversion
            Intrinsic::ToString => {
                self.compile_util_to_string_call(&call.arguments, function, locals)
            }

            // String utilities
            Intrinsic::StringIndexOf => {
                self.compile_string_index_of_call(&call.arguments, function, locals)
            }
            Intrinsic::StringSplit => {
                self.compile_string_split_call(&call.arguments, function, locals)
            }
            Intrinsic::StringContains => {
                self.compile_string_contains_call(&call.arguments, function, locals)
            }
            Intrinsic::StringReplace => {
                self.compile_string_replace_call(&call.arguments, function, locals)
            }

            // Assertions
            Intrinsic::Fail => self.compile_fail_call(&call.arguments, function, locals),
            Intrinsic::AssertSnapshot => {
                bail!("__intrinsic_assert_snapshot is not implemented yet")
            }

            // Environment
            Intrinsic::EnvGet => self.compile_env_get_call(&call.arguments, function, locals),
            Intrinsic::EnvSet => self.compile_env_set_call(&call.arguments, function, locals),
            Intrinsic::EnvVars => self.compile_env_vars_call(&call.arguments, function, locals),
            Intrinsic::EnvCwd => self.compile_env_cwd_call(&call.arguments, function, locals),

            // Filesystem
            Intrinsic::FsReadText => {
                self.compile_fs_read_text_call(&call.arguments, function, locals)
            }
            Intrinsic::FsWriteText => {
                self.compile_fs_write_text_call(&call.arguments, function, locals)
            }

            Intrinsic::FsCreateDir => {
                self.compile_fs_create_dir_call(&call.arguments, function, locals)
            }
            Intrinsic::FsRemove => self.compile_fs_remove_call(&call.arguments, function, locals),
            Intrinsic::FsListDir => {
                self.compile_fs_list_dir_call(&call.arguments, function, locals)
            }
            // Path
            Intrinsic::PathJoin => self.compile_path_join_call(&call.arguments, function, locals),
            Intrinsic::PathComponents => {
                self.compile_path_components_call(&call.arguments, function, locals)
            }
            Intrinsic::PathDirname => {
                self.compile_path_dirname_call(&call.arguments, function, locals)
            }
            Intrinsic::PathBasename => {
                self.compile_path_basename_call(&call.arguments, function, locals)
            }
            Intrinsic::PathExtension => {
                self.compile_path_extension_call(&call.arguments, function, locals)
            } // Process
              // Codecs (removed - now handled by runtime)
              // CLI
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
        let left_value = self.expr_to_tea_value(left_expr)?.into_struct_value();
        let right_value = self.expr_to_tea_value(right_expr)?.into_struct_value();

        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;

        // Pass by pointer for ARM64 ABI compatibility
        let left_alloca =
            map_builder_error(self.builder.build_alloca(tea_value_type, "assert_eq_left"))?;
        map_builder_error(self.builder.build_store(left_alloca, left_value))?;

        let right_alloca =
            map_builder_error(self.builder.build_alloca(tea_value_type, "assert_eq_right"))?;
        map_builder_error(self.builder.build_store(right_alloca, right_value))?;

        let func = self.ensure_assert_eq_fn();
        self.call_function(
            func,
            &[left_alloca.into(), right_alloca.into()],
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
        let left_value = self.expr_to_tea_value(left_expr)?.into_struct_value();
        let right_value = self.expr_to_tea_value(right_expr)?.into_struct_value();

        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;

        // Pass by pointer for ARM64 ABI compatibility
        let left_alloca =
            map_builder_error(self.builder.build_alloca(tea_value_type, "assert_ne_left"))?;
        map_builder_error(self.builder.build_store(left_alloca, left_value))?;

        let right_alloca =
            map_builder_error(self.builder.build_alloca(tea_value_type, "assert_ne_right"))?;
        map_builder_error(self.builder.build_store(right_alloca, right_value))?;

        let func = self.ensure_assert_ne_fn();
        self.call_function(
            func,
            &[left_alloca.into(), right_alloca.into()],
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

        // Fast path: inline length access for strings and lists
        match &value_expr {
            ExprValue::String(ptr) => {
                // TeaString: { tag: i8, len: i8, data: [22 x i8] }
                // Length is in field 1 as i8
                let string_type = self
                    .context
                    .get_struct_type("TeaString")
                    .ok_or_else(|| anyhow!("TeaString type not found"))?;
                let len_ptr = map_builder_error(self.builder.build_struct_gep(
                    string_type,
                    *ptr,
                    1,
                    "len_ptr",
                ))?;
                let len_i8 = map_builder_error(self.builder.build_load(
                    self.context.i8_type(),
                    len_ptr,
                    "len_i8",
                ))?
                .into_int_value();
                let length = map_builder_error(self.builder.build_int_z_extend(
                    len_i8,
                    self.int_type(),
                    "len",
                ))?;
                return Ok(ExprValue::Int(length));
            }
            ExprValue::List { pointer: ptr, .. } => {
                // TeaList: { tag: i8, len: i8, padding: [6 x i8], data: [8 x TeaValue] }
                // For inline lists (tag=1): length is in field 1 as i8
                // For heap lists (tag=0): length is in first 8 bytes of data array
                // Use FFI to handle both cases correctly
                let func = self.ensure_list_len_ffi_fn();
                let length = self
                    .call_function(func, &[(*ptr).into()], "tea_list_len_ffi")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("tea_list_len_ffi returned no value"))?
                    .into_int_value();
                return Ok(ExprValue::Int(length));
            }
            _ => {
                // Slow path: fall back to FFI for dicts and other types
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
        }
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
        let tea_value = self.expr_to_tea_value(value_expr)?.into_struct_value();
        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;
        // Pass by pointer for ARM64 ABI compatibility
        let alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "tea_value_tmp"))?;
        map_builder_error(self.builder.build_store(alloca, tea_value))?;
        let func = self.ensure_util_to_string_fn();
        let pointer = self
            .call_function(func, &[alloca.into()], "tea_util_to_string")?
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
        // Type predicates have been removed from the stdlib
        let _ = (arguments, function, locals, kind);
        bail!("unsupported util predicate - type predicates have been removed")
    }

    fn compile_string_index_of_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("string_index_of is not supported by the LLVM backend yet")
    }

    fn compile_string_split_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("string_split is not supported by the LLVM backend yet")
    }

    fn compile_string_contains_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("string_contains is not supported by the LLVM backend yet")
    }

    fn compile_string_replace_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("string_replace is not supported by the LLVM backend yet")
    }

    fn compile_string_to_lower_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("string_to_lower is not supported by the LLVM backend yet")
    }

    fn compile_string_to_upper_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("string_to_upper is not supported by the LLVM backend yet")
    }

    fn compile_math_floor_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("floor expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for floor");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let float_val = match value_expr {
            ExprValue::Float(v) => v,
            _ => bail!("floor expects a Float argument"),
        };

        // Use LLVM's floor intrinsic
        let floor_intrinsic = inkwell::intrinsics::Intrinsic::find("llvm.floor")
            .ok_or_else(|| anyhow!("llvm.floor intrinsic not found"))?;
        let intrinsic_fn = floor_intrinsic
            .get_declaration(&self.module, &[self.float_type().into()])
            .ok_or_else(|| anyhow!("failed to get llvm.floor declaration"))?;

        let floor_result = self
            .call_function(intrinsic_fn, &[float_val.into()], "floor_result")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("floor returned no value"))?
            .into_float_value();

        // Convert float to int
        let int_result = map_builder_error(self.builder.build_float_to_signed_int(
            floor_result,
            self.int_type(),
            "floor_to_int",
        ))?;

        Ok(ExprValue::Int(int_result))
    }

    fn compile_math_ceil_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("ceil expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for ceil");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let float_val = match value_expr {
            ExprValue::Float(v) => v,
            _ => bail!("ceil expects a Float argument"),
        };

        // Use LLVM's ceil intrinsic
        let ceil_intrinsic = inkwell::intrinsics::Intrinsic::find("llvm.ceil")
            .ok_or_else(|| anyhow!("llvm.ceil intrinsic not found"))?;
        let intrinsic_fn = ceil_intrinsic
            .get_declaration(&self.module, &[self.float_type().into()])
            .ok_or_else(|| anyhow!("failed to get llvm.ceil declaration"))?;

        let ceil_result = self
            .call_function(intrinsic_fn, &[float_val.into()], "ceil_result")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("ceil returned no value"))?
            .into_float_value();

        // Convert float to int
        let int_result = map_builder_error(self.builder.build_float_to_signed_int(
            ceil_result,
            self.int_type(),
            "ceil_to_int",
        ))?;

        Ok(ExprValue::Int(int_result))
    }

    fn compile_math_round_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("round expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for round");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let float_val = match value_expr {
            ExprValue::Float(v) => v,
            _ => bail!("round expects a Float argument"),
        };

        // Use LLVM's round intrinsic (rounds to nearest even)
        let round_intrinsic = inkwell::intrinsics::Intrinsic::find("llvm.round")
            .ok_or_else(|| anyhow!("llvm.round intrinsic not found"))?;
        let intrinsic_fn = round_intrinsic
            .get_declaration(&self.module, &[self.float_type().into()])
            .ok_or_else(|| anyhow!("failed to get llvm.round declaration"))?;

        let round_result = self
            .call_function(intrinsic_fn, &[float_val.into()], "round_result")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("round returned no value"))?
            .into_float_value();

        // Convert float to int
        let int_result = map_builder_error(self.builder.build_float_to_signed_int(
            round_result,
            self.int_type(),
            "round_to_int",
        ))?;

        Ok(ExprValue::Int(int_result))
    }

    fn compile_math_abs_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("abs expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for abs");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let float_val = match value_expr {
            ExprValue::Float(v) => v,
            _ => bail!("abs expects a Float argument"),
        };

        // Use LLVM's fabs intrinsic
        let abs_intrinsic = inkwell::intrinsics::Intrinsic::find("llvm.fabs")
            .ok_or_else(|| anyhow!("llvm.fabs intrinsic not found"))?;
        let intrinsic_fn = abs_intrinsic
            .get_declaration(&self.module, &[self.float_type().into()])
            .ok_or_else(|| anyhow!("failed to get llvm.fabs declaration"))?;

        let abs_result = self
            .call_function(intrinsic_fn, &[float_val.into()], "abs_result")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("abs returned no value"))?
            .into_float_value();

        Ok(ExprValue::Float(abs_result))
    }

    fn compile_math_sqrt_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("sqrt expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for sqrt");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let float_val = match value_expr {
            ExprValue::Float(v) => v,
            _ => bail!("sqrt expects a Float argument"),
        };

        // Use LLVM's sqrt intrinsic
        let sqrt_intrinsic = inkwell::intrinsics::Intrinsic::find("llvm.sqrt")
            .ok_or_else(|| anyhow!("llvm.sqrt intrinsic not found"))?;
        let intrinsic_fn = sqrt_intrinsic
            .get_declaration(&self.module, &[self.float_type().into()])
            .ok_or_else(|| anyhow!("failed to get llvm.sqrt declaration"))?;

        let sqrt_result = self
            .call_function(intrinsic_fn, &[float_val.into()], "sqrt_result")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("sqrt returned no value"))?
            .into_float_value();

        Ok(ExprValue::Float(sqrt_result))
    }

    fn compile_math_min_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("min expects exactly 2 arguments");
        }
        if arguments[0].name.is_some() || arguments[1].name.is_some() {
            bail!("named arguments are not supported for min");
        }
        let a_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let b_expr = self.compile_expression(&arguments[1].expression, function, locals)?;

        let (a_float, b_float) = match (a_expr, b_expr) {
            (ExprValue::Float(a), ExprValue::Float(b)) => (a, b),
            _ => bail!("min expects two Float arguments"),
        };

        let func = self.ensure_fmin_fn();
        let result = self
            .call_function(func, &[a_float.into(), b_float.into()], "fmin")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("fmin returned no value"))?
            .into_float_value();

        Ok(ExprValue::Float(result))
    }

    fn compile_math_max_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("max expects exactly 2 arguments");
        }
        if arguments[0].name.is_some() || arguments[1].name.is_some() {
            bail!("named arguments are not supported for max");
        }
        let a_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let b_expr = self.compile_expression(&arguments[1].expression, function, locals)?;

        let (a_float, b_float) = match (a_expr, b_expr) {
            (ExprValue::Float(a), ExprValue::Float(b)) => (a, b),
            _ => bail!("max expects two Float arguments"),
        };

        let func = self.ensure_fmax_fn();
        let result = self
            .call_function(func, &[a_float.into(), b_float.into()], "fmax")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("fmax returned no value"))?
            .into_float_value();

        Ok(ExprValue::Float(result))
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

    // Environment directory getters (no-arg -> string)
    compile_noarg_string_call!(
        compile_env_cwd_call,
        ensure_env_cwd_fn,
        "tea_env_cwd",
        "env.cwd"
    );
    compile_noarg_string_call!(
        compile_env_temp_dir_call,
        ensure_env_temp_dir_fn,
        "tea_env_temp_dir",
        "env.temp_dir"
    );
    compile_noarg_string_call!(
        compile_env_home_dir_call,
        ensure_env_home_dir_fn,
        "tea_env_home_dir",
        "env.home_dir"
    );
    compile_noarg_string_call!(
        compile_env_config_dir_call,
        ensure_env_config_dir_fn,
        "tea_env_config_dir",
        "env.config_dir"
    );

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

    // Path string transforms (string -> string)
    compile_string_to_string_call!(
        compile_path_dirname_call,
        ensure_path_dirname_fn,
        "tea_path_dirname",
        "path.dirname"
    );
    compile_string_to_string_call!(
        compile_path_basename_call,
        ensure_path_basename_fn,
        "tea_path_basename",
        "path.basename"
    );
    compile_string_to_string_call!(
        compile_path_extension_call,
        ensure_path_extension_fn,
        "tea_path_extension",
        "path.extension"
    );

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

    compile_string_to_string_call!(
        compile_path_strip_extension_call,
        ensure_path_strip_extension_fn,
        "tea_path_strip_extension",
        "path.strip_extension"
    );
    compile_string_to_string_call!(
        compile_path_normalize_call,
        ensure_path_normalize_fn,
        "tea_path_normalize",
        "path.normalize"
    );

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

        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;

        let command_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let command_ptr = self.expect_string_pointer(
            command_expr,
            "process.run expects the command to be a String",
        )?;

        // Build args TeaValue and allocate on stack for ARM64 ABI compatibility
        let args_tea_value = if arguments.len() >= 2 {
            let expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let args_alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "args_ptr"))?;
        map_builder_error(self.builder.build_store(args_alloca, args_tea_value))?;

        // Build env TeaValue and allocate on stack
        let env_tea_value = if arguments.len() >= 3 {
            let expr = self.compile_expression(&arguments[2].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let env_alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "env_ptr"))?;
        map_builder_error(self.builder.build_store(env_alloca, env_tea_value))?;

        // Build cwd TeaValue and allocate on stack
        let cwd_tea_value = if arguments.len() >= 4 {
            let expr = self.compile_expression(&arguments[3].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let cwd_alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "cwd_ptr"))?;
        map_builder_error(self.builder.build_store(cwd_alloca, cwd_tea_value))?;

        // Build stdin TeaValue and allocate on stack
        let stdin_tea_value = if arguments.len() >= 5 {
            let expr = self.compile_expression(&arguments[4].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let stdin_alloca =
            map_builder_error(self.builder.build_alloca(tea_value_type, "stdin_ptr"))?;
        map_builder_error(self.builder.build_store(stdin_alloca, stdin_tea_value))?;

        let template_ptr = self.ensure_struct_template("ProcessResult")?;
        let func = self.ensure_process_run_fn();
        let pointer = self
            .call_function(
                func,
                &[
                    template_ptr.into(),
                    command_ptr.as_basic_value_enum().into(),
                    args_alloca.into(),
                    env_alloca.into(),
                    cwd_alloca.into(),
                    stdin_alloca.into(),
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

        let tea_value_type = self
            .context
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;

        let command_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let command_ptr = self.expect_string_pointer(
            command_expr,
            "process.spawn expects the command to be a String",
        )?;

        // Build args TeaValue and allocate on stack for ARM64 ABI compatibility
        let args_tea_value = if arguments.len() >= 2 {
            let expr = self.compile_expression(&arguments[1].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let args_alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "args_ptr"))?;
        map_builder_error(self.builder.build_store(args_alloca, args_tea_value))?;

        // Build env TeaValue and allocate on stack
        let env_tea_value = if arguments.len() >= 3 {
            let expr = self.compile_expression(&arguments[2].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let env_alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "env_ptr"))?;
        map_builder_error(self.builder.build_store(env_alloca, env_tea_value))?;

        // Build cwd TeaValue and allocate on stack
        let cwd_tea_value = if arguments.len() >= 4 {
            let expr = self.compile_expression(&arguments[3].expression, function, locals)?;
            self.expr_to_tea_value(expr)?
        } else {
            let nil_fn = self.ensure_value_nil();
            self.call_function(nil_fn, &[], "val_nil")?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected TeaValue"))?
        };
        let cwd_alloca = map_builder_error(self.builder.build_alloca(tea_value_type, "cwd_ptr"))?;
        map_builder_error(self.builder.build_store(cwd_alloca, cwd_tea_value))?;

        let func = self.ensure_process_spawn_fn();
        let value = self
            .call_function(
                func,
                &[
                    command_ptr.as_basic_value_enum().into(),
                    args_alloca.into(),
                    env_alloca.into(),
                    cwd_alloca.into(),
                ],
                "tea_process_spawn",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_process_spawn returned no value"))?
            .into_int_value();

        Ok(ExprValue::Int(value))
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

    fn compile_process_read_stdout_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        self.compile_process_read_call(arguments, function, locals, true)
    }

    fn compile_process_read_stderr_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        self.compile_process_read_call(arguments, function, locals, false)
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

    fn compile_fs_rename_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("fs_rename is not supported by the LLVM backend yet")
    }

    fn compile_fs_stat_call(
        &mut self,
        _arguments: &[crate::ast::CallArgument],
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        bail!("fs_stat is not supported by the LLVM backend yet")
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

    /// Compile a List method call (map, filter, reduce, find, any, all)
    fn compile_list_method_call(
        &mut self,
        method_name: &str,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match method_name {
            "map" => self.compile_list_map(list_ptr, element_type, call, function, locals),
            "filter" => self.compile_list_filter(list_ptr, element_type, call, function, locals),
            "reduce" => self.compile_list_reduce(list_ptr, element_type, call, function, locals),
            "find" => self.compile_list_find(list_ptr, element_type, call, function, locals),
            "any" => self.compile_list_any(list_ptr, element_type, call, function, locals),
            "all" => self.compile_list_all(list_ptr, element_type, call, function, locals),
            _ => bail!("unknown List method: {}", method_name),
        }
    }

    /// Compile list.map(fn) - transform each element
    fn compile_list_map(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Get the callback closure
        let callback = self.compile_expression(&call.arguments[0].expression, function, locals)?;
        let (closure_ptr, param_types, return_type) = match callback {
            ExprValue::Closure {
                pointer,
                param_types,
                return_type,
            } => (pointer, param_types, return_type),
            _ => bail!("List.map expects a function argument"),
        };

        // Get list length
        let list_len_fn = self.ensure_list_len_ffi_fn();
        let length = map_builder_error(self.builder.build_call(
            list_len_fn,
            &[list_ptr.into()],
            "list_len",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected i64 from list_len"))?
        .into_int_value();

        // Allocate result list
        let alloc_list_fn = self.ensure_alloc_list();
        let result_list = map_builder_error(self.builder.build_call(
            alloc_list_fn,
            &[length.into()],
            "result_list",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected pointer from alloc_list"))?
        .into_pointer_value();

        // Create loop blocks
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;
        let cond_block = self.context.append_basic_block(function, "map_cond");
        let body_block = self.context.append_basic_block(function, "map_body");
        let exit_block = self.context.append_basic_block(function, "map_exit");

        // Branch to condition
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let i64_type = self.context.i64_type();
        let index_phi = map_builder_error(self.builder.build_phi(i64_type, "index"))?;
        index_phi.add_incoming(&[(&i64_type.const_zero(), current_block)]);

        let cond = map_builder_error(self.builder.build_int_compare(
            IntPredicate::SLT,
            index_phi.as_basic_value().into_int_value(),
            length,
            "map_cond",
        ))?;
        map_builder_error(
            self.builder
                .build_conditional_branch(cond, body_block, exit_block),
        )?;

        // Body block
        self.builder.position_at_end(body_block);

        // Get element at index
        let list_get_fn = self.ensure_list_get();
        let tea_value = map_builder_error(self.builder.build_call(
            list_get_fn,
            &[list_ptr.into(), index_phi.as_basic_value().into()],
            "element_value",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected TeaValue"))?
        .into_struct_value();

        // Convert to proper type
        let element = self.tea_value_to_expr_value(tea_value, &element_type)?;

        // Call closure with element
        let call_result = self.call_closure_with_args(
            closure_ptr,
            &param_types,
            &return_type,
            vec![element],
            function,
        )?;

        // Convert result to TeaValue and store in result list
        let result_tea_value = self.expr_to_tea_value(call_result)?;
        let list_set_fn = self.ensure_list_set();
        map_builder_error(self.builder.build_call(
            list_set_fn,
            &[
                result_list.into(),
                index_phi.as_basic_value().into(),
                result_tea_value.into(),
            ],
            "",
        ))?;

        // Increment index
        let next_index = map_builder_error(self.builder.build_int_add(
            index_phi.as_basic_value().into_int_value(),
            i64_type.const_int(1, false),
            "next_index",
        ))?;
        index_phi.add_incoming(&[(&next_index, body_block)]);
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Exit block
        self.builder.position_at_end(exit_block);

        Ok(ExprValue::List {
            pointer: result_list,
            element_type: return_type,
        })
    }

    /// Compile list.filter(fn) - keep elements matching predicate
    fn compile_list_filter(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Get the callback closure
        let callback = self.compile_expression(&call.arguments[0].expression, function, locals)?;
        let (closure_ptr, param_types, _return_type) = match callback {
            ExprValue::Closure {
                pointer,
                param_types,
                return_type,
            } => (pointer, param_types, return_type),
            _ => bail!("List.filter expects a function argument"),
        };

        // Get list length
        let list_len_fn = self.ensure_list_len_ffi_fn();
        let length = map_builder_error(self.builder.build_call(
            list_len_fn,
            &[list_ptr.into()],
            "list_len",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected i64 from list_len"))?
        .into_int_value();

        // Allocate result list (start with same capacity, will be smaller or equal)
        let alloc_list_fn = self.ensure_alloc_list();
        let result_list = map_builder_error(self.builder.build_call(
            alloc_list_fn,
            &[self.context.i64_type().const_zero().into()],
            "result_list",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected pointer from alloc_list"))?
        .into_pointer_value();

        // Create loop blocks
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;
        let cond_block = self.context.append_basic_block(function, "filter_cond");
        let body_block = self.context.append_basic_block(function, "filter_body");
        let append_block = self.context.append_basic_block(function, "filter_append");
        let continue_block = self.context.append_basic_block(function, "filter_continue");
        let exit_block = self.context.append_basic_block(function, "filter_exit");

        // Branch to condition
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let i64_type = self.context.i64_type();
        let index_phi = map_builder_error(self.builder.build_phi(i64_type, "index"))?;
        index_phi.add_incoming(&[(&i64_type.const_zero(), current_block)]);

        let cond = map_builder_error(self.builder.build_int_compare(
            IntPredicate::SLT,
            index_phi.as_basic_value().into_int_value(),
            length,
            "filter_cond",
        ))?;
        map_builder_error(
            self.builder
                .build_conditional_branch(cond, body_block, exit_block),
        )?;

        // Body block
        self.builder.position_at_end(body_block);

        // Get element at index
        let list_get_fn = self.ensure_list_get();
        let tea_value = map_builder_error(self.builder.build_call(
            list_get_fn,
            &[list_ptr.into(), index_phi.as_basic_value().into()],
            "element_value",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected TeaValue"))?
        .into_struct_value();

        // Convert to proper type
        let element = self.tea_value_to_expr_value(tea_value, &element_type)?;

        // Call closure with element
        let predicate_result = self.call_closure_with_args(
            closure_ptr,
            &param_types,
            &Box::new(ValueType::Bool),
            vec![element.clone()],
            function,
        )?;

        // Check predicate result
        let bool_val = match predicate_result {
            ExprValue::Bool(v) => v,
            _ => bail!("filter predicate must return Bool"),
        };

        // Compare with true (1)
        let is_true = map_builder_error(self.builder.build_int_compare(
            IntPredicate::NE,
            bool_val,
            self.context.i32_type().const_zero(),
            "is_true",
        ))?;

        map_builder_error(self.builder.build_conditional_branch(
            is_true,
            append_block,
            continue_block,
        ))?;

        // Append block - add element to result
        self.builder.position_at_end(append_block);
        let append_fn = self.ensure_list_append_fn();
        let element_tea_value = self.expr_to_tea_value(element)?;
        map_builder_error(self.builder.build_call(
            append_fn,
            &[result_list.into(), element_tea_value.into()],
            "",
        ))?;
        map_builder_error(self.builder.build_unconditional_branch(continue_block))?;

        // Continue block - increment and loop
        self.builder.position_at_end(continue_block);
        let next_index = map_builder_error(self.builder.build_int_add(
            index_phi.as_basic_value().into_int_value(),
            i64_type.const_int(1, false),
            "next_index",
        ))?;
        index_phi.add_incoming(&[(&next_index, continue_block)]);
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Exit block
        self.builder.position_at_end(exit_block);

        Ok(ExprValue::List {
            pointer: result_list,
            element_type,
        })
    }

    /// Compile list.reduce(init, fn) - fold list to single value
    fn compile_list_reduce(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Get initial value
        let init_value =
            self.compile_expression(&call.arguments[0].expression, function, locals)?;
        let init_type = init_value.ty();

        // Get the callback closure
        let callback = self.compile_expression(&call.arguments[1].expression, function, locals)?;
        let (closure_ptr, param_types, return_type) = match callback {
            ExprValue::Closure {
                pointer,
                param_types,
                return_type,
            } => (pointer, param_types, return_type),
            _ => bail!("List.reduce expects a function argument"),
        };

        // Get list length
        let list_len_fn = self.ensure_list_len_ffi_fn();
        let length = map_builder_error(self.builder.build_call(
            list_len_fn,
            &[list_ptr.into()],
            "list_len",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected i64 from list_len"))?
        .into_int_value();

        // Store accumulator in alloca for updates within loop
        let acc_alloca =
            self.create_entry_alloca(function, "reduce_acc", self.basic_type(&init_type)?)?;
        let init_basic = init_value
            .into_basic_value()
            .ok_or_else(|| anyhow!("init value must produce a basic value"))?;
        map_builder_error(self.builder.build_store(acc_alloca, init_basic))?;

        // Create loop blocks
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;
        let cond_block = self.context.append_basic_block(function, "reduce_cond");
        let body_block = self.context.append_basic_block(function, "reduce_body");
        let exit_block = self.context.append_basic_block(function, "reduce_exit");

        // Branch to condition
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let i64_type = self.context.i64_type();
        let index_phi = map_builder_error(self.builder.build_phi(i64_type, "index"))?;
        index_phi.add_incoming(&[(&i64_type.const_zero(), current_block)]);

        let cond = map_builder_error(self.builder.build_int_compare(
            IntPredicate::SLT,
            index_phi.as_basic_value().into_int_value(),
            length,
            "reduce_cond",
        ))?;
        map_builder_error(
            self.builder
                .build_conditional_branch(cond, body_block, exit_block),
        )?;

        // Body block
        self.builder.position_at_end(body_block);

        // Load current accumulator
        let acc_basic = map_builder_error(self.builder.build_load(
            self.basic_type(&init_type)?,
            acc_alloca,
            "acc_load",
        ))?;
        let acc_expr = self.basic_to_expr_value(acc_basic, &init_type)?;

        // Get element at index
        let list_get_fn = self.ensure_list_get();
        let tea_value = map_builder_error(self.builder.build_call(
            list_get_fn,
            &[list_ptr.into(), index_phi.as_basic_value().into()],
            "element_value",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected TeaValue"))?
        .into_struct_value();

        // Convert to proper type
        let element = self.tea_value_to_expr_value(tea_value, &element_type)?;

        // Call closure with accumulator and element
        let new_acc = self.call_closure_with_args(
            closure_ptr,
            &param_types,
            &return_type,
            vec![acc_expr, element],
            function,
        )?;

        // Store new accumulator
        let new_acc_basic = new_acc
            .into_basic_value()
            .ok_or_else(|| anyhow!("reducer must return a value"))?;
        map_builder_error(self.builder.build_store(acc_alloca, new_acc_basic))?;

        // Increment index
        let next_index = map_builder_error(self.builder.build_int_add(
            index_phi.as_basic_value().into_int_value(),
            i64_type.const_int(1, false),
            "next_index",
        ))?;
        index_phi.add_incoming(&[(&next_index, body_block)]);
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Exit block
        self.builder.position_at_end(exit_block);
        let final_acc = map_builder_error(self.builder.build_load(
            self.basic_type(&init_type)?,
            acc_alloca,
            "final_acc",
        ))?;
        self.basic_to_expr_value(final_acc, &init_type)
    }

    /// Compile list.find(fn) - first matching element (nil if none)
    fn compile_list_find(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Get the callback closure
        let callback = self.compile_expression(&call.arguments[0].expression, function, locals)?;
        let (closure_ptr, param_types, _return_type) = match callback {
            ExprValue::Closure {
                pointer,
                param_types,
                return_type,
            } => (pointer, param_types, return_type),
            _ => bail!("List.find expects a function argument"),
        };

        // Get list length
        let list_len_fn = self.ensure_list_len_ffi_fn();
        let length = map_builder_error(self.builder.build_call(
            list_len_fn,
            &[list_ptr.into()],
            "list_len",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected i64 from list_len"))?
        .into_int_value();

        // Create result alloca for optional value
        let result_alloca =
            map_builder_error(self.builder.build_alloca(self.value_type(), "find_result"))?;
        // Initialize with nil
        let value_nil_fn = self.ensure_value_nil();
        let nil_value = map_builder_error(self.builder.build_call(value_nil_fn, &[], "nil_value"))?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("expected TeaValue from value_nil"))?;
        map_builder_error(self.builder.build_store(result_alloca, nil_value))?;

        // Create loop blocks
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;
        let cond_block = self.context.append_basic_block(function, "find_cond");
        let body_block = self.context.append_basic_block(function, "find_body");
        let found_block = self.context.append_basic_block(function, "find_found");
        let continue_block = self.context.append_basic_block(function, "find_continue");
        let exit_block = self.context.append_basic_block(function, "find_exit");

        // Branch to condition
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let i64_type = self.context.i64_type();
        let index_phi = map_builder_error(self.builder.build_phi(i64_type, "index"))?;
        index_phi.add_incoming(&[(&i64_type.const_zero(), current_block)]);

        let cond = map_builder_error(self.builder.build_int_compare(
            IntPredicate::SLT,
            index_phi.as_basic_value().into_int_value(),
            length,
            "find_cond",
        ))?;
        map_builder_error(
            self.builder
                .build_conditional_branch(cond, body_block, exit_block),
        )?;

        // Body block
        self.builder.position_at_end(body_block);

        // Get element at index
        let list_get_fn = self.ensure_list_get();
        let tea_value = map_builder_error(self.builder.build_call(
            list_get_fn,
            &[list_ptr.into(), index_phi.as_basic_value().into()],
            "element_value",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected TeaValue"))?
        .into_struct_value();

        // Store tea_value for potential use in found_block
        let element_alloca =
            map_builder_error(self.builder.build_alloca(self.value_type(), "element_temp"))?;
        map_builder_error(self.builder.build_store(element_alloca, tea_value))?;

        // Convert to proper type
        let element = self.tea_value_to_expr_value(tea_value, &element_type)?;

        // Call closure with element
        let predicate_result = self.call_closure_with_args(
            closure_ptr,
            &param_types,
            &Box::new(ValueType::Bool),
            vec![element],
            function,
        )?;

        // Check predicate result
        let bool_val = match predicate_result {
            ExprValue::Bool(v) => v,
            _ => bail!("find predicate must return Bool"),
        };

        let is_true = map_builder_error(self.builder.build_int_compare(
            IntPredicate::NE,
            bool_val,
            self.context.i32_type().const_zero(),
            "is_true",
        ))?;

        map_builder_error(self.builder.build_conditional_branch(
            is_true,
            found_block,
            continue_block,
        ))?;

        // Found block - store element and exit
        self.builder.position_at_end(found_block);
        let found_element = map_builder_error(self.builder.build_load(
            self.value_type(),
            element_alloca,
            "found_element",
        ))?;
        map_builder_error(self.builder.build_store(result_alloca, found_element))?;
        map_builder_error(self.builder.build_unconditional_branch(exit_block))?;

        // Continue block - increment and loop
        self.builder.position_at_end(continue_block);
        let next_index = map_builder_error(self.builder.build_int_add(
            index_phi.as_basic_value().into_int_value(),
            i64_type.const_int(1, false),
            "next_index",
        ))?;
        index_phi.add_incoming(&[(&next_index, continue_block)]);
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Exit block
        self.builder.position_at_end(exit_block);
        let final_result = map_builder_error(self.builder.build_load(
            self.value_type(),
            result_alloca,
            "final_result",
        ))?
        .into_struct_value();

        Ok(ExprValue::Optional {
            value: final_result,
            inner: element_type,
        })
    }

    /// Compile list.any(fn) - true if any element matches
    fn compile_list_any(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        self.compile_list_any_all(list_ptr, element_type, call, function, locals, true)
    }

    /// Compile list.all(fn) - true if all elements match
    fn compile_list_all(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        self.compile_list_any_all(list_ptr, element_type, call, function, locals, false)
    }

    /// Shared implementation for any/all
    fn compile_list_any_all(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        element_type: Box<ValueType>,
        call: &CallExpression,
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
        is_any: bool,
    ) -> Result<ExprValue<'ctx>> {
        // Get the callback closure
        let callback = self.compile_expression(&call.arguments[0].expression, function, locals)?;
        let (closure_ptr, param_types, _return_type) = match callback {
            ExprValue::Closure {
                pointer,
                param_types,
                return_type,
            } => (pointer, param_types, return_type),
            _ => bail!(
                "List.{} expects a function argument",
                if is_any { "any" } else { "all" }
            ),
        };

        // Get list length
        let list_len_fn = self.ensure_list_len_ffi_fn();
        let length = map_builder_error(self.builder.build_call(
            list_len_fn,
            &[list_ptr.into()],
            "list_len",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected i64 from list_len"))?
        .into_int_value();

        // Create result alloca
        let i32_type = self.context.i32_type();
        let result_alloca =
            map_builder_error(self.builder.build_alloca(i32_type, "any_all_result"))?;
        // Initialize: any starts with false (0), all starts with true (1)
        let initial = if is_any {
            i32_type.const_zero()
        } else {
            i32_type.const_int(1, false)
        };
        map_builder_error(self.builder.build_store(result_alloca, initial))?;

        // Create loop blocks
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| anyhow!("missing insertion block"))?;
        let cond_block = self.context.append_basic_block(function, "any_all_cond");
        let body_block = self.context.append_basic_block(function, "any_all_body");
        let early_exit_block = self
            .context
            .append_basic_block(function, "any_all_early_exit");
        let continue_block = self
            .context
            .append_basic_block(function, "any_all_continue");
        let exit_block = self.context.append_basic_block(function, "any_all_exit");

        // Branch to condition
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Condition block
        self.builder.position_at_end(cond_block);
        let i64_type = self.context.i64_type();
        let index_phi = map_builder_error(self.builder.build_phi(i64_type, "index"))?;
        index_phi.add_incoming(&[(&i64_type.const_zero(), current_block)]);

        let cond = map_builder_error(self.builder.build_int_compare(
            IntPredicate::SLT,
            index_phi.as_basic_value().into_int_value(),
            length,
            "any_all_cond",
        ))?;
        map_builder_error(
            self.builder
                .build_conditional_branch(cond, body_block, exit_block),
        )?;

        // Body block
        self.builder.position_at_end(body_block);

        // Get element at index
        let list_get_fn = self.ensure_list_get();
        let tea_value = map_builder_error(self.builder.build_call(
            list_get_fn,
            &[list_ptr.into(), index_phi.as_basic_value().into()],
            "element_value",
        ))?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("expected TeaValue"))?
        .into_struct_value();

        // Convert to proper type
        let element = self.tea_value_to_expr_value(tea_value, &element_type)?;

        // Call closure with element
        let predicate_result = self.call_closure_with_args(
            closure_ptr,
            &param_types,
            &Box::new(ValueType::Bool),
            vec![element],
            function,
        )?;

        // Check predicate result
        let bool_val = match predicate_result {
            ExprValue::Bool(v) => v,
            _ => bail!("predicate must return Bool"),
        };

        let is_true = map_builder_error(self.builder.build_int_compare(
            IntPredicate::NE,
            bool_val,
            i32_type.const_zero(),
            "is_true",
        ))?;

        // For any: if true, early exit with true
        // For all: if false, early exit with false
        if is_any {
            map_builder_error(self.builder.build_conditional_branch(
                is_true,
                early_exit_block,
                continue_block,
            ))?;
        } else {
            map_builder_error(self.builder.build_conditional_branch(
                is_true,
                continue_block,
                early_exit_block,
            ))?;
        }

        // Early exit block
        self.builder.position_at_end(early_exit_block);
        let early_result = if is_any {
            i32_type.const_int(1, false) // true for any
        } else {
            i32_type.const_zero() // false for all
        };
        map_builder_error(self.builder.build_store(result_alloca, early_result))?;
        map_builder_error(self.builder.build_unconditional_branch(exit_block))?;

        // Continue block
        self.builder.position_at_end(continue_block);
        let next_index = map_builder_error(self.builder.build_int_add(
            index_phi.as_basic_value().into_int_value(),
            i64_type.const_int(1, false),
            "next_index",
        ))?;
        index_phi.add_incoming(&[(&next_index, continue_block)]);
        map_builder_error(self.builder.build_unconditional_branch(cond_block))?;

        // Exit block
        self.builder.position_at_end(exit_block);
        let final_result = map_builder_error(self.builder.build_load(
            i32_type,
            result_alloca,
            "final_result",
        ))?
        .into_int_value();

        Ok(ExprValue::Bool(final_result))
    }

    /// Helper to call a closure with pre-compiled arguments
    fn call_closure_with_args(
        &mut self,
        closure_ptr: PointerValue<'ctx>,
        param_types: &[ValueType],
        return_type: &Box<ValueType>,
        args: Vec<ExprValue<'ctx>>,
        function: FunctionValue<'ctx>,
    ) -> Result<ExprValue<'ctx>> {
        let mut arg_values: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len());
        for (index, (expected_type, arg)) in param_types.iter().zip(args.into_iter()).enumerate() {
            let converted = self
                .convert_expr_to_type(arg, expected_type)
                .map_err(|error| {
                    anyhow!("argument {} has mismatched type: {}", index + 1, error)
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

        let fn_type = match return_type.as_ref() {
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

        let mut call_args: Vec<BasicMetadataValueEnum<'ctx>> =
            Vec::with_capacity(arg_values.len() + 1);
        call_args.push(closure_ptr.into());
        for value in arg_values {
            call_args.push(BasicMetadataValueEnum::from(value));
        }

        let call_site = map_builder_error(self.builder.build_indirect_call(
            fn_type,
            typed_fn_ptr,
            &call_args,
            "closure_call",
        ))?;

        if matches!(return_type.as_ref(), ValueType::Void) {
            self.handle_possible_error(function)?;
            return Ok(ExprValue::Void);
        }

        let result = call_site
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("closure returned no value"))?;

        let expr = match return_type.as_ref() {
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

    /// Helper to convert BasicValueEnum to ExprValue
    fn basic_to_expr_value(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &ValueType,
    ) -> Result<ExprValue<'ctx>> {
        match ty {
            ValueType::Int => Ok(ExprValue::Int(value.into_int_value())),
            ValueType::Float => Ok(ExprValue::Float(value.into_float_value())),
            ValueType::Bool => Ok(ExprValue::Bool(value.into_int_value())),
            ValueType::String => Ok(ExprValue::String(value.into_pointer_value())),
            ValueType::List(inner) => Ok(ExprValue::List {
                pointer: value.into_pointer_value(),
                element_type: inner.clone(),
            }),
            ValueType::Dict(inner) => Ok(ExprValue::Dict {
                pointer: value.into_pointer_value(),
                value_type: inner.clone(),
            }),
            ValueType::Struct(name) => Ok(ExprValue::Struct {
                pointer: value.into_pointer_value(),
                struct_name: name.clone(),
            }),
            ValueType::Error {
                error_name,
                variant_name,
            } => Ok(ExprValue::Error {
                pointer: value.into_pointer_value(),
                error_name: error_name.clone(),
                variant_name: variant_name.clone(),
            }),
            ValueType::Function(params, ret) => Ok(ExprValue::Closure {
                pointer: value.into_pointer_value(),
                param_types: params.clone(),
                return_type: ret.clone(),
            }),
            ValueType::Optional(inner) => Ok(ExprValue::Optional {
                value: value.into_struct_value(),
                inner: inner.clone(),
            }),
            ValueType::Void => bail!("cannot convert void to ExprValue"),
        }
    }

    /// Compile a Dict method call (keys, values, entries)
    fn compile_dict_method_call(
        &mut self,
        method_name: &str,
        dict_ptr: PointerValue<'ctx>,
        value_type: Box<ValueType>,
        _call: &CallExpression,
        _function: FunctionValue<'ctx>,
        _locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        match method_name {
            "keys" => {
                let dict_keys_fn = self.ensure_dict_keys_fn();
                let keys_ptr = map_builder_error(self.builder.build_call(
                    dict_keys_fn,
                    &[dict_ptr.into()],
                    "dict_keys",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected pointer from dict_keys"))?
                .into_pointer_value();
                Ok(ExprValue::List {
                    pointer: keys_ptr,
                    element_type: Box::new(ValueType::String),
                })
            }
            "values" => {
                let dict_values_fn = self.ensure_dict_values_fn();
                let values_ptr = map_builder_error(self.builder.build_call(
                    dict_values_fn,
                    &[dict_ptr.into()],
                    "dict_values",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected pointer from dict_values"))?
                .into_pointer_value();
                Ok(ExprValue::List {
                    pointer: values_ptr,
                    element_type: value_type,
                })
            }
            "entries" => {
                let dict_entries_fn = self.ensure_dict_entries_fn();
                let entries_ptr = map_builder_error(self.builder.build_call(
                    dict_entries_fn,
                    &[dict_ptr.into()],
                    "dict_entries",
                ))?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| anyhow!("expected pointer from dict_entries"))?
                .into_pointer_value();
                // Each entry is a dict with the same value type
                Ok(ExprValue::List {
                    pointer: entries_ptr,
                    element_type: Box::new(ValueType::Dict(value_type)),
                })
            }
            _ => bail!("unknown Dict method: {}", method_name),
        }
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
                // Pass TeaValue by pointer to avoid ARM64 ABI issues
                let tea_value = self.expr_to_tea_value(converted)?.into_struct_value();
                let tea_value_type = self
                    .context
                    .get_struct_type("TeaValue")
                    .ok_or_else(|| anyhow!("TeaValue type not found"))?;
                let tea_value_alloca = map_builder_error(
                    self.builder
                        .build_alloca(tea_value_type, "struct_field_value"),
                )?;
                map_builder_error(self.builder.build_store(tea_value_alloca, tea_value))?;
                self.call_function(
                    set_fn,
                    &[
                        struct_ptr.into(),
                        self.int_type().const_int(index as u64, false).into(),
                        tea_value_alloca.into(),
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
                // Pass TeaValue by pointer to avoid ARM64 ABI issues
                let tea_value = self.expr_to_tea_value(converted)?.into_struct_value();
                let tea_value_type = self
                    .context
                    .get_struct_type("TeaValue")
                    .ok_or_else(|| anyhow!("TeaValue type not found"))?;
                let tea_value_alloca = map_builder_error(
                    self.builder
                        .build_alloca(tea_value_type, "struct_field_value"),
                )?;
                map_builder_error(self.builder.build_store(tea_value_alloca, tea_value))?;
                self.call_function(
                    set_fn,
                    &[
                        struct_ptr.into(),
                        self.int_type().const_int(index as u64, false).into(),
                        tea_value_alloca.into(),
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
            // Pass TeaValue by pointer to avoid ARM64 ABI issues
            let tea_value = self.expr_to_tea_value(converted)?.into_struct_value();
            let tea_value_type = self
                .context
                .get_struct_type("TeaValue")
                .ok_or_else(|| anyhow!("TeaValue type not found"))?;
            let tea_value_alloca = map_builder_error(
                self.builder
                    .build_alloca(tea_value_type, "error_field_value"),
            )?;
            map_builder_error(self.builder.build_store(tea_value_alloca, tea_value))?;
            self.call_function(
                set_fn,
                &[
                    error_ptr.into(),
                    self.int_type().const_int(index as u64, false).into(),
                    tea_value_alloca.into(),
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
                    let tea_value_type = self
                        .context
                        .get_struct_type("TeaValue")
                        .ok_or_else(|| anyhow!("TeaValue type not found"))?;
                    // Pass by pointer for ARM64 ABI compatibility
                    let alloca = map_builder_error(
                        self.builder.build_alloca(tea_value_type, "opt_tea_val_tmp"),
                    )?;
                    map_builder_error(self.builder.build_store(alloca, value))?;
                    let to_string = self.ensure_util_to_string_fn();
                    let string_ptr = self
                        .call_function(to_string, &[alloca.into()], "optional_to_string")?
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

    fn compile_println_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Same as print but calls println functions instead
        for argument in arguments {
            if argument.name.is_some() {
                bail!("named arguments are not supported by the LLVM backend yet");
            }
            let value = self.compile_expression(&argument.expression, function, locals)?;
            match value {
                ExprValue::Int(v) => {
                    let func = self.ensure_println_int();
                    self.call_function(func, &[v.into()], "println_int")?;
                }
                ExprValue::Float(v) => {
                    let func = self.ensure_println_float();
                    self.call_function(func, &[v.into()], "println_float")?;
                }
                ExprValue::Bool(v) => {
                    let cast = self.bool_to_i32(v, "println_bool")?;
                    let func = self.ensure_println_bool();
                    self.call_function(func, &[cast.into()], "println_bool")?;
                }
                ExprValue::String(ptr) => {
                    let func = self.ensure_println_string();
                    self.call_function(func, &[ptr.into()], "println_string")?;
                }
                ExprValue::List { pointer, .. } => {
                    let func = self.ensure_println_list();
                    self.call_function(func, &[pointer.into()], "println_list")?;
                }
                ExprValue::Dict { pointer, .. } => {
                    let func = self.ensure_println_dict();
                    self.call_function(func, &[pointer.into()], "println_dict")?;
                }
                ExprValue::Struct { pointer, .. } => {
                    let func = self.ensure_println_struct();
                    self.call_function(func, &[pointer.into()], "println_struct")?;
                }
                ExprValue::Error { pointer, .. } => {
                    let func = self.ensure_println_error();
                    self.call_function(func, &[pointer.into()], "println_error")?;
                }
                ExprValue::Closure { pointer, .. } => {
                    let func = self.ensure_println_closure();
                    self.call_function(func, &[pointer.into()], "println_closure")?;
                }
                ExprValue::Optional { value, .. } => {
                    let tea_value_type = self
                        .context
                        .get_struct_type("TeaValue")
                        .ok_or_else(|| anyhow!("TeaValue type not found"))?;
                    // Pass by pointer for ARM64 ABI compatibility
                    let alloca = map_builder_error(
                        self.builder.build_alloca(tea_value_type, "opt_tea_val_tmp"),
                    )?;
                    map_builder_error(self.builder.build_store(alloca, value))?;
                    let to_string = self.ensure_util_to_string_fn();
                    let string_ptr = self
                        .call_function(to_string, &[alloca.into()], "optional_to_string")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("tea_util_to_string returned no value"))?
                        .into_pointer_value();
                    let func = self.ensure_println_string();
                    self.call_function(func, &[string_ptr.into()], "println_optional")?;
                }
                ExprValue::Void => {
                    let nil_string = self.compile_string_literal("nil")?;
                    if let ExprValue::String(ptr) = nil_string {
                        let func = self.ensure_println_string();
                        self.call_function(func, &[ptr.into()], "println_nil")?;
                    } else {
                        bail!("expected string literal for nil printing");
                    }
                }
            }
        }
        Ok(ExprValue::Void)
    }

    fn compile_to_string_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Same as UtilToString
        self.compile_util_to_string_call(arguments, function, locals)
    }

    fn compile_type_of_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("type_of expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for type_of");
        }
        let value_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let tea_value = self.expr_to_tea_value(value_expr)?;
        let func = self.ensure_type_of_fn();
        let pointer = self
            .call_function(func, &[tea_value.into()], "tea_type_of")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_type_of returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_panic_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("panic expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for panic");
        }
        let message_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let message_ptr = match message_expr {
            ExprValue::String(ptr) => ptr,
            _ => bail!("panic expects a String argument"),
        };
        let func = self.ensure_panic_fn();
        self.call_function(func, &[message_ptr.into()], "tea_panic")?;
        Ok(ExprValue::Void)
    }

    fn compile_exit_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("exit expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for exit");
        }
        let code_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let code = match code_expr {
            ExprValue::Int(val) => val,
            _ => bail!("exit expects an Int argument"),
        };
        let func = self.ensure_exit_fn();
        self.call_function(func, &[code.into()], "tea_exit")?;
        Ok(ExprValue::Void)
    }

    fn compile_args_call(&mut self) -> Result<ExprValue<'ctx>> {
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

    fn compile_args_program_call(&mut self) -> Result<ExprValue<'ctx>> {
        let func = self.ensure_args_program_fn();
        let pointer = self
            .call_function(func, &[], "tea_args_program")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_args_program returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    // Regex functions
    fn compile_regex_compile_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 1 {
            bail!("regex.compile expects exactly 1 argument");
        }
        if arguments[0].name.is_some() {
            bail!("named arguments are not supported for regex.compile");
        }
        let pattern_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let pattern_ptr = self.expect_string_pointer(
            pattern_expr,
            "regex.compile expects the pattern to be a String",
        )?;
        let func = self.ensure_regex_compile_fn();
        let handle = self
            .call_function(func, &[pattern_ptr.into()], "tea_regex_compile")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_compile returned no value"))?
            .into_int_value();
        Ok(ExprValue::Int(handle))
    }

    fn compile_regex_is_match_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("regex.is_match expects exactly 2 arguments");
        }
        for arg in arguments {
            if arg.name.is_some() {
                bail!("named arguments are not supported for regex.is_match");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "regex.is_match expects the handle to be an Int",
        )?;
        let text_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let text_ptr = self
            .expect_string_pointer(text_expr, "regex.is_match expects the text to be a String")?;
        let func = self.ensure_regex_is_match_fn();
        let raw = self
            .call_function(
                func,
                &[handle_value.into(), text_ptr.into()],
                "tea_regex_is_match",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_is_match returned no value"))?
            .into_int_value();
        let bool_value = self.i32_to_bool(raw, "regex_is_match_result")?;
        Ok(ExprValue::Bool(bool_value))
    }

    fn compile_regex_find_all_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("regex.find_all expects exactly 2 arguments");
        }
        for arg in arguments {
            if arg.name.is_some() {
                bail!("named arguments are not supported for regex.find_all");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "regex.find_all expects the handle to be an Int",
        )?;
        let text_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let text_ptr = self
            .expect_string_pointer(text_expr, "regex.find_all expects the text to be a String")?;
        let func = self.ensure_regex_find_all_fn();
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), text_ptr.into()],
                "tea_regex_find_all",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_find_all returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_regex_captures_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("regex.captures expects exactly 2 arguments");
        }
        for arg in arguments {
            if arg.name.is_some() {
                bail!("named arguments are not supported for regex.captures");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "regex.captures expects the handle to be an Int",
        )?;
        let text_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let text_ptr = self
            .expect_string_pointer(text_expr, "regex.captures expects the text to be a String")?;
        let func = self.ensure_regex_captures_fn();
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), text_ptr.into()],
                "tea_regex_captures",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_captures returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_regex_replace_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 3 {
            bail!("regex.replace expects exactly 3 arguments");
        }
        for arg in arguments {
            if arg.name.is_some() {
                bail!("named arguments are not supported for regex.replace");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value =
            self.expect_int_value(handle_expr, "regex.replace expects the handle to be an Int")?;
        let text_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let text_ptr =
            self.expect_string_pointer(text_expr, "regex.replace expects the text to be a String")?;
        let replacement_expr =
            self.compile_expression(&arguments[2].expression, function, locals)?;
        let replacement_ptr = self.expect_string_pointer(
            replacement_expr,
            "regex.replace expects the replacement to be a String",
        )?;
        let func = self.ensure_regex_replace_fn();
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), text_ptr.into(), replacement_ptr.into()],
                "tea_regex_replace",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_replace returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_regex_replace_all_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 3 {
            bail!("regex.replace_all expects exactly 3 arguments");
        }
        for arg in arguments {
            if arg.name.is_some() {
                bail!("named arguments are not supported for regex.replace_all");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value = self.expect_int_value(
            handle_expr,
            "regex.replace_all expects the handle to be an Int",
        )?;
        let text_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let text_ptr = self.expect_string_pointer(
            text_expr,
            "regex.replace_all expects the text to be a String",
        )?;
        let replacement_expr =
            self.compile_expression(&arguments[2].expression, function, locals)?;
        let replacement_ptr = self.expect_string_pointer(
            replacement_expr,
            "regex.replace_all expects the replacement to be a String",
        )?;
        let func = self.ensure_regex_replace_all_fn();
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), text_ptr.into(), replacement_ptr.into()],
                "tea_regex_replace_all",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_replace_all returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_regex_split_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        if arguments.len() != 2 {
            bail!("regex.split expects exactly 2 arguments");
        }
        for arg in arguments {
            if arg.name.is_some() {
                bail!("named arguments are not supported for regex.split");
            }
        }
        let handle_expr = self.compile_expression(&arguments[0].expression, function, locals)?;
        let handle_value =
            self.expect_int_value(handle_expr, "regex.split expects the handle to be an Int")?;
        let text_expr = self.compile_expression(&arguments[1].expression, function, locals)?;
        let text_ptr =
            self.expect_string_pointer(text_expr, "regex.split expects the text to be a String")?;
        let func = self.ensure_regex_split_fn();
        let pointer = self
            .call_function(
                func,
                &[handle_value.into(), text_ptr.into()],
                "tea_regex_split",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_regex_split returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::List {
            pointer,
            element_type: Box::new(ValueType::String),
        })
    }

    fn compile_read_line_call(&mut self) -> Result<ExprValue<'ctx>> {
        let func = self.ensure_read_line_fn();
        let pointer = self
            .call_function(func, &[], "tea_read_line")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_read_line returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_read_all_call(&mut self) -> Result<ExprValue<'ctx>> {
        let func = self.ensure_read_all_fn();
        let pointer = self
            .call_function(func, &[], "tea_read_all")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_read_all returned no value"))?
            .into_pointer_value();
        Ok(ExprValue::String(pointer))
    }

    fn compile_eprint_call(
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
                    let func = self.ensure_eprint_int();
                    self.call_function(func, &[v.into()], "eprint_int")?;
                }
                ExprValue::Float(v) => {
                    let func = self.ensure_eprint_float();
                    self.call_function(func, &[v.into()], "eprint_float")?;
                }
                ExprValue::Bool(v) => {
                    let cast = self.bool_to_i32(v, "eprint_bool")?;
                    let func = self.ensure_eprint_bool();
                    self.call_function(func, &[cast.into()], "eprint_bool")?;
                }
                ExprValue::String(ptr) => {
                    let func = self.ensure_eprint_string();
                    self.call_function(func, &[ptr.into()], "eprint_string")?;
                }
                ExprValue::List { pointer, .. } => {
                    let func = self.ensure_eprint_list();
                    self.call_function(func, &[pointer.into()], "eprint_list")?;
                }
                ExprValue::Dict { pointer, .. } => {
                    let func = self.ensure_eprint_dict();
                    self.call_function(func, &[pointer.into()], "eprint_dict")?;
                }
                ExprValue::Struct { pointer, .. } => {
                    let func = self.ensure_eprint_struct();
                    self.call_function(func, &[pointer.into()], "eprint_struct")?;
                }
                ExprValue::Error { pointer, .. } => {
                    let func = self.ensure_eprint_error();
                    self.call_function(func, &[pointer.into()], "eprint_error")?;
                }
                ExprValue::Closure { pointer, .. } => {
                    let func = self.ensure_eprint_closure();
                    self.call_function(func, &[pointer.into()], "eprint_closure")?;
                }
                ExprValue::Void | ExprValue::Optional { .. } => {
                    bail!("cannot eprint void or optional values");
                }
            }
        }
        Ok(ExprValue::Void)
    }

    fn compile_eprintln_call(
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
                    let func = self.ensure_eprintln_int();
                    self.call_function(func, &[v.into()], "eprintln_int")?;
                }
                ExprValue::Float(v) => {
                    let func = self.ensure_eprintln_float();
                    self.call_function(func, &[v.into()], "eprintln_float")?;
                }
                ExprValue::Bool(v) => {
                    let cast = self.bool_to_i32(v, "eprintln_bool")?;
                    let func = self.ensure_eprintln_bool();
                    self.call_function(func, &[cast.into()], "eprintln_bool")?;
                }
                ExprValue::String(ptr) => {
                    let func = self.ensure_eprintln_string();
                    self.call_function(func, &[ptr.into()], "eprintln_string")?;
                }
                ExprValue::List { pointer, .. } => {
                    let func = self.ensure_eprintln_list();
                    self.call_function(func, &[pointer.into()], "eprintln_list")?;
                }
                ExprValue::Dict { pointer, .. } => {
                    let func = self.ensure_eprintln_dict();
                    self.call_function(func, &[pointer.into()], "eprintln_dict")?;
                }
                ExprValue::Struct { pointer, .. } => {
                    let func = self.ensure_eprintln_struct();
                    self.call_function(func, &[pointer.into()], "eprintln_struct")?;
                }
                ExprValue::Error { pointer, .. } => {
                    let func = self.ensure_eprintln_error();
                    self.call_function(func, &[pointer.into()], "eprintln_error")?;
                }
                ExprValue::Closure { pointer, .. } => {
                    let func = self.ensure_eprintln_closure();
                    self.call_function(func, &[pointer.into()], "eprintln_closure")?;
                }
                ExprValue::Void | ExprValue::Optional { .. } => {
                    bail!("cannot eprintln void or optional values");
                }
            }
        }
        Ok(ExprValue::Void)
    }

    fn compile_is_tty_call(&mut self) -> Result<ExprValue<'ctx>> {
        let func = self.ensure_is_tty_fn();
        let result = self
            .call_function(func, &[], "tea_is_tty")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_is_tty returned no value"))?
            .into_int_value();
        // Convert i32 (0 or 1) to bool
        let zero = self.context.i32_type().const_int(0, false);
        let bool_val = map_builder_error(self.builder.build_int_compare(
            inkwell::IntPredicate::NE,
            result,
            zero,
            "is_tty_bool",
        ))?;
        Ok(ExprValue::Bool(bool_val))
    }

    fn compile_length_call(
        &mut self,
        arguments: &[crate::ast::CallArgument],
        function: FunctionValue<'ctx>,
        locals: &mut HashMap<String, LocalVariable<'ctx>>,
    ) -> Result<ExprValue<'ctx>> {
        // Reuse the existing util_len_call logic
        self.compile_util_len_call(arguments, function, locals)
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
            (ExprValue::String(lhs), ExprValue::String(rhs)) => {
                let result = self.concat_string_values(lhs, rhs)?;
                Ok(ExprValue::String(result))
            }
            (
                ExprValue::List {
                    pointer: lhs_ptr,
                    element_type: lhs_elem,
                },
                ExprValue::List {
                    pointer: rhs_ptr,
                    element_type: _rhs_elem,
                },
            ) => {
                // For now, assume element types match (typechecker ensures this)
                let result = self.concat_list_values(lhs_ptr, rhs_ptr)?;
                Ok(ExprValue::List {
                    pointer: result,
                    element_type: lhs_elem,
                })
            }
            _ => bail!("add expects numeric, string, or list operands"),
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
        // Inline construction of TeaValue to avoid ABI issues with struct returns.
        // TeaValue is { tag: i32, padding: i32, payload: i64 } - 16 bytes total
        // Explicit padding ensures consistent layout with Rust #[repr(C)] struct
        // Tags: Int=0, Float=1, Bool=2, String=3, List=4, Dict=5, Struct=6, Error=7, Closure=8, Nil=9
        let tea_value_type = self
            .module
            .get_struct_type("TeaValue")
            .ok_or_else(|| anyhow!("TeaValue type not found"))?;

        match value {
            ExprValue::Int(v) => build_tea_value(
                &self.context,
                &self.builder,
                tea_value_type,
                TeaValueTag::Int,
                v,
                "int",
            ),
            ExprValue::Float(v) => {
                let payload = map_builder_error(self.builder.build_bit_cast(
                    v,
                    self.context.i64_type(),
                    "float_bits",
                ))?
                .into_int_value();
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::Float,
                    payload,
                    "float",
                )
            }
            ExprValue::Bool(v) => {
                let payload = map_builder_error(self.builder.build_int_z_extend(
                    v,
                    self.context.i64_type(),
                    "bool_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::Bool,
                    payload,
                    "bool",
                )
            }
            ExprValue::String(ptr) => {
                let payload = map_builder_error(self.builder.build_ptr_to_int(
                    ptr,
                    self.context.i64_type(),
                    "ptr_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::String,
                    payload,
                    "string",
                )
            }
            ExprValue::List { pointer, .. } => {
                let payload = map_builder_error(self.builder.build_ptr_to_int(
                    pointer,
                    self.context.i64_type(),
                    "ptr_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::List,
                    payload,
                    "list",
                )
            }
            ExprValue::Dict { pointer, .. } => {
                let payload = map_builder_error(self.builder.build_ptr_to_int(
                    pointer,
                    self.context.i64_type(),
                    "ptr_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::Dict,
                    payload,
                    "dict",
                )
            }
            ExprValue::Struct { pointer, .. } => {
                let payload = map_builder_error(self.builder.build_ptr_to_int(
                    pointer,
                    self.context.i64_type(),
                    "ptr_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::Struct,
                    payload,
                    "struct",
                )
            }
            ExprValue::Error { pointer, .. } => {
                let payload = map_builder_error(self.builder.build_ptr_to_int(
                    pointer,
                    self.context.i64_type(),
                    "ptr_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::Error,
                    payload,
                    "error",
                )
            }
            ExprValue::Closure { pointer, .. } => {
                let payload = map_builder_error(self.builder.build_ptr_to_int(
                    pointer,
                    self.context.i64_type(),
                    "ptr_i64",
                ))?;
                build_tea_value(
                    &self.context,
                    &self.builder,
                    tea_value_type,
                    TeaValueTag::Closure,
                    payload,
                    "closure",
                )
            }
            ExprValue::Void => {
                // Tag = 9 (Nil), payload = 0
                // Use const_named_struct for compile-time constant (more efficient)
                let tag = self.context.i32_type().const_int(9, false);
                let padding = self.context.i32_type().const_zero();
                let payload = self.context.i64_type().const_zero();
                let struct_val = tea_value_type.const_named_struct(&[
                    tag.into(),
                    padding.into(),
                    payload.into(),
                ]);
                Ok(struct_val.into())
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
                    pointer: Some(tmp_alloca),
                    value: None,
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
                let tea_value = self.expr_to_tea_value(other)?.into_struct_value();
                let tea_value_type = self
                    .context
                    .get_struct_type("TeaValue")
                    .ok_or_else(|| anyhow!("TeaValue type not found"))?;

                // ARM64 ABI fix: Pass TeaValue by pointer instead of by value
                // This avoids struct passing issues with the C ABI on ARM64
                let alloca =
                    map_builder_error(self.builder.build_alloca(tea_value_type, "tea_value_tmp"))?;
                map_builder_error(self.builder.build_store(alloca, tea_value))?;

                let func = self.ensure_util_to_string_fn();
                let call = self
                    .call_function(func, &[alloca.into()], "tea_util_to_string")?
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
        // Call runtime tea_string_concat which handles the new tagged union structure
        let concat_fn = self.ensure_string_concat_fn();
        let result = self
            .call_function(concat_fn, &[left.into(), right.into()], "concat_str")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_string_concat returned no value"))?
            .into_pointer_value();

        Ok(result)
    }

    /// Push string onto target, mutating in place. Returns possibly reallocated target.
    fn push_string_value(
        &mut self,
        target: PointerValue<'ctx>,
        src: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let push_fn = self.ensure_string_push_str_fn();
        let result = self
            .call_function(push_fn, &[target.into(), src.into()], "push_str")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_string_push_str returned no value"))?
            .into_pointer_value();
        Ok(result)
    }

    fn concat_list_values(
        &mut self,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        // Call tea_list_concat runtime function
        let concat_fn = self.ensure_list_concat_fn();
        let result = self
            .call_function(concat_fn, &[left.into(), right.into()], "list_concat")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("tea_list_concat returned no value"))?
            .into_pointer_value();
        Ok(result)
    }

    /// Convert a TeaValue payload (i64) to an ExprValue based on the expected type.
    fn payload_to_expr(
        &mut self,
        payload: IntValue<'ctx>,
        ty: ValueType,
    ) -> Result<ExprValue<'ctx>> {
        match ty {
            ValueType::Int => {
                // Payload is the int value directly
                Ok(ExprValue::Int(payload))
            }
            ValueType::Float => {
                // Payload is the float value bitcast from i64
                let float_val = map_builder_error(self.builder.build_bit_cast(
                    payload,
                    self.float_type(),
                    "float_val",
                ))?
                .into_float_value();
                Ok(ExprValue::Float(float_val))
            }
            ValueType::Bool => {
                // Payload is 0 or 1 as i64, convert to i1
                let bool_val = map_builder_error(self.builder.build_int_compare(
                    inkwell::IntPredicate::NE,
                    payload,
                    self.context.i64_type().const_zero(),
                    "bool_val",
                ))?;
                Ok(ExprValue::Bool(bool_val))
            }
            ValueType::String => {
                // Payload is a pointer stored as i64
                let ptr = map_builder_error(self.builder.build_int_to_ptr(
                    payload,
                    self.ptr_type,
                    "string_ptr",
                ))?;
                Ok(ExprValue::String(ptr))
            }
            ValueType::List(inner) => {
                // Payload is a pointer stored as i64
                let ptr = map_builder_error(self.builder.build_int_to_ptr(
                    payload,
                    self.ptr_type,
                    "list_ptr",
                ))?;
                Ok(ExprValue::List {
                    pointer: ptr,
                    element_type: inner,
                })
            }
            ValueType::Dict(inner) => {
                // Payload is a pointer stored as i64
                let ptr = map_builder_error(self.builder.build_int_to_ptr(
                    payload,
                    self.ptr_type,
                    "dict_ptr",
                ))?;
                Ok(ExprValue::Dict {
                    pointer: ptr,
                    value_type: inner,
                })
            }
            ValueType::Struct(struct_name) => {
                // Payload is a pointer stored as i64
                let ptr = map_builder_error(self.builder.build_int_to_ptr(
                    payload,
                    self.ptr_type,
                    "struct_ptr",
                ))?;
                Ok(ExprValue::Struct {
                    pointer: ptr,
                    struct_name,
                })
            }
            ValueType::Error {
                error_name,
                variant_name,
            } => {
                // Payload is a pointer stored as i64
                let ptr = map_builder_error(self.builder.build_int_to_ptr(
                    payload,
                    self.ptr_type,
                    "error_ptr",
                ))?;
                Ok(ExprValue::Error {
                    pointer: ptr,
                    error_name,
                    variant_name,
                })
            }
            ValueType::Function(param_types, return_type) => {
                // Payload is a pointer stored as i64
                let ptr = map_builder_error(self.builder.build_int_to_ptr(
                    payload,
                    self.ptr_type,
                    "closure_ptr",
                ))?;
                Ok(ExprValue::Closure {
                    pointer: ptr,
                    param_types,
                    return_type,
                })
            }
            ValueType::Optional(_) | ValueType::Void => {
                bail!("payload_to_expr doesn't handle Optional or Void")
            }
        }
    }

    fn tea_value_to_expr(
        &mut self,
        value: StructValue<'ctx>,
        ty: ValueType,
    ) -> Result<ExprValue<'ctx>> {
        // Handle Optional specially since it needs the full TeaValue
        if let ValueType::Optional(inner) = ty {
            return Ok(ExprValue::Optional { value, inner });
        }
        if ty == ValueType::Void {
            return Ok(ExprValue::Void);
        }

        // Extract payload directly using extractvalue instruction
        // TeaValue is { tag: i32, padding: i32, payload: i64 } - payload is at index 2
        let payload = map_builder_error(self.builder.build_extract_value(value, 2, "payload"))?
            .into_int_value();

        self.payload_to_expr(payload, ty)
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

    // Print functions (print without newline)
    define_ffi_print_fn!(ensure_print_int, builtin_print_int, "tea_print_int", int);
    define_ffi_print_fn!(
        ensure_print_float,
        builtin_print_float,
        "tea_print_float",
        float
    );
    define_ffi_print_fn!(
        ensure_print_bool,
        builtin_print_bool,
        "tea_print_bool",
        bool
    );
    define_ffi_print_fn!(
        ensure_print_string,
        builtin_print_string,
        "tea_print_string",
        string
    );
    define_ffi_print_fn!(
        ensure_print_list,
        builtin_print_list,
        "tea_print_list",
        list
    );
    define_ffi_print_fn!(
        ensure_print_dict,
        builtin_print_dict,
        "tea_print_dict",
        dict
    );
    define_ffi_print_fn!(
        ensure_print_struct,
        builtin_print_struct,
        "tea_print_struct",
        struct
    );
    define_ffi_print_fn!(
        ensure_print_error,
        builtin_print_error,
        "tea_print_error",
        error
    );
    define_ffi_print_fn!(
        ensure_print_closure,
        builtin_print_closure,
        "tea_print_closure",
        closure
    );

    // Println functions (print with newline)
    define_ffi_print_fn!(
        ensure_println_int,
        builtin_println_int,
        "tea_println_int",
        int
    );
    define_ffi_print_fn!(
        ensure_println_float,
        builtin_println_float,
        "tea_println_float",
        float
    );
    define_ffi_print_fn!(
        ensure_println_bool,
        builtin_println_bool,
        "tea_println_bool",
        bool
    );
    define_ffi_print_fn!(
        ensure_println_string,
        builtin_println_string,
        "tea_println_string",
        string
    );
    define_ffi_print_fn!(
        ensure_println_list,
        builtin_println_list,
        "tea_println_list",
        list
    );
    define_ffi_print_fn!(
        ensure_println_dict,
        builtin_println_dict,
        "tea_println_dict",
        dict
    );
    define_ffi_print_fn!(
        ensure_println_struct,
        builtin_println_struct,
        "tea_println_struct",
        struct
    );
    define_ffi_print_fn!(
        ensure_println_error,
        builtin_println_error,
        "tea_println_error",
        error
    );
    define_ffi_print_fn!(
        ensure_println_closure,
        builtin_println_closure,
        "tea_println_closure",
        closure
    );

    // Eprint functions (print to stderr without newline)
    define_ffi_print_fn!(ensure_eprint_int, eprint_int_fn, "tea_eprint_int", int);
    define_ffi_print_fn!(
        ensure_eprint_float,
        eprint_float_fn,
        "tea_eprint_float",
        float
    );
    define_ffi_print_fn!(ensure_eprint_bool, eprint_bool_fn, "tea_eprint_bool", bool);
    define_ffi_print_fn!(
        ensure_eprint_string,
        eprint_string_fn,
        "tea_eprint_string",
        string
    );
    define_ffi_print_fn!(ensure_eprint_list, eprint_list_fn, "tea_eprint_list", list);
    define_ffi_print_fn!(ensure_eprint_dict, eprint_dict_fn, "tea_eprint_dict", dict);
    define_ffi_print_fn!(
        ensure_eprint_struct,
        eprint_struct_fn,
        "tea_eprint_struct",
        struct
    );
    define_ffi_print_fn!(
        ensure_eprint_error,
        eprint_error_fn,
        "tea_eprint_error",
        error
    );
    define_ffi_print_fn!(
        ensure_eprint_closure,
        eprint_closure_fn,
        "tea_eprint_closure",
        closure
    );

    // Eprintln functions (print to stderr with newline)
    define_ffi_print_fn!(
        ensure_eprintln_int,
        eprintln_int_fn,
        "tea_eprintln_int",
        int
    );
    define_ffi_print_fn!(
        ensure_eprintln_float,
        eprintln_float_fn,
        "tea_eprintln_float",
        float
    );
    define_ffi_print_fn!(
        ensure_eprintln_bool,
        eprintln_bool_fn,
        "tea_eprintln_bool",
        bool
    );
    define_ffi_print_fn!(
        ensure_eprintln_string,
        eprintln_string_fn,
        "tea_eprintln_string",
        string
    );
    define_ffi_print_fn!(
        ensure_eprintln_list,
        eprintln_list_fn,
        "tea_eprintln_list",
        list
    );
    define_ffi_print_fn!(
        ensure_eprintln_dict,
        eprintln_dict_fn,
        "tea_eprintln_dict",
        dict
    );
    define_ffi_print_fn!(
        ensure_eprintln_struct,
        eprintln_struct_fn,
        "tea_eprintln_struct",
        struct
    );
    define_ffi_print_fn!(
        ensure_eprintln_error,
        eprintln_error_fn,
        "tea_eprintln_error",
        error
    );
    define_ffi_print_fn!(
        ensure_eprintln_closure,
        eprintln_closure_fn,
        "tea_eprintln_closure",
        closure
    );

    fn ensure_type_of_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_type_of_fn {
            return func;
        }
        let tea_value_type = self.value_type();
        let fn_type = self
            .string_ptr_type()
            .fn_type(&[tea_value_type.into()], false);
        let func = self
            .module
            .add_function("tea_type_of", fn_type, Some(Linkage::External));
        self.builtin_type_of_fn = Some(func);
        func
    }

    fn ensure_panic_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_panic_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_panic", fn_type, Some(Linkage::External));
        self.builtin_panic_fn = Some(func);
        func
    }

    fn ensure_exit_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_exit_fn {
            return func;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("tea_exit", fn_type, Some(Linkage::External));
        self.builtin_exit_fn = Some(func);
        func
    }

    fn ensure_dict_delete_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_dict_delete_fn {
            return func;
        }
        let fn_type = self.dict_ptr_type().fn_type(
            &[self.dict_ptr_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_dict_delete", fn_type, Some(Linkage::External));
        self.builtin_dict_delete_fn = Some(func);
        func
    }

    fn ensure_dict_clear_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_dict_clear_fn {
            return func;
        }
        let fn_type = self
            .dict_ptr_type()
            .fn_type(&[self.dict_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_dict_clear", fn_type, Some(Linkage::External));
        self.builtin_dict_clear_fn = Some(func);
        func
    }

    fn ensure_fmax_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_fmax_fn {
            return func;
        }
        let fn_type = self
            .float_type()
            .fn_type(&[self.float_type().into(), self.float_type().into()], false);
        let func = self
            .module
            .add_function("fmax", fn_type, Some(Linkage::External));
        self.builtin_fmax_fn = Some(func);
        func
    }

    fn ensure_fmin_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_fmin_fn {
            return func;
        }
        let fn_type = self
            .float_type()
            .fn_type(&[self.float_type().into(), self.float_type().into()], false);
        let func = self
            .module
            .add_function("fmin", fn_type, Some(Linkage::External));
        self.builtin_fmin_fn = Some(func);
        func
    }

    fn ensure_list_append_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.builtin_list_append_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(
            &[self.list_ptr_type().into(), self.value_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_list_append", fn_type, Some(Linkage::External));
        self.builtin_list_append_fn = Some(func);
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
        // Pass TeaValue by pointer for ARM64 ABI compatibility
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[ptr_type.into(), ptr_type.into()], false);
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
        // Pass TeaValue by pointer for ARM64 ABI compatibility
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[ptr_type.into(), ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.string_ptr_type().fn_type(&[ptr_type.into()], false);
        let func = self
            .module
            .add_function("tea_util_to_string", fn_type, Some(Linkage::External));
        self.util_to_string_fn = Some(func);
        func
    }

    fn ensure_malloc_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.malloc_fn {
            return func;
        }
        let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.int_type().into()], false);
        let func = self
            .module
            .add_function("malloc", fn_type, Some(Linkage::External));
        self.malloc_fn = Some(func);
        func
    }

    fn ensure_memcpy_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.memcpy_fn {
            return func;
        }
        let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(
            &[
                i8_ptr_type.into(),
                i8_ptr_type.into(),
                self.int_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("memcpy", fn_type, Some(Linkage::External));
        self.memcpy_fn = Some(func);
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

    // Type checking utilities
    define_ffi_typecheck_fn!(ensure_util_is_nil_fn, util_is_nil_fn, "tea_util_is_nil");
    define_ffi_typecheck_fn!(ensure_util_is_bool_fn, util_is_bool_fn, "tea_util_is_bool");
    define_ffi_typecheck_fn!(ensure_util_is_int_fn, util_is_int_fn, "tea_util_is_int");
    define_ffi_typecheck_fn!(
        ensure_util_is_float_fn,
        util_is_float_fn,
        "tea_util_is_float"
    );
    define_ffi_typecheck_fn!(
        ensure_util_is_string_fn,
        util_is_string_fn,
        "tea_util_is_string"
    );
    define_ffi_typecheck_fn!(ensure_util_is_list_fn, util_is_list_fn, "tea_util_is_list");
    define_ffi_typecheck_fn!(
        ensure_util_is_struct_fn,
        util_is_struct_fn,
        "tea_util_is_struct"
    );
    define_ffi_typecheck_fn!(
        ensure_util_is_error_fn,
        util_is_error_fn,
        "tea_util_is_error"
    );

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

    // Environment directory getters
    define_ffi_string_getter_fn!(ensure_env_cwd_fn, env_cwd_fn, "tea_env_cwd");
    define_ffi_string_getter_fn!(ensure_env_temp_dir_fn, env_temp_dir_fn, "tea_env_temp_dir");
    define_ffi_string_getter_fn!(ensure_env_home_dir_fn, env_home_dir_fn, "tea_env_home_dir");
    define_ffi_string_getter_fn!(
        ensure_env_config_dir_fn,
        env_config_dir_fn,
        "tea_env_config_dir"
    );

    // Environment setters
    define_ffi_string_setter_fn!(ensure_env_set_cwd_fn, env_set_cwd_fn, "tea_env_set_cwd");

    // Path functions
    define_ffi_list_to_string_fn!(ensure_path_join_fn, path_join_fn, "tea_path_join");
    define_ffi_string_to_list_fn!(
        ensure_path_components_fn,
        path_components_fn,
        "tea_path_components"
    );
    define_ffi_string_transform_fn!(ensure_path_dirname_fn, path_dirname_fn, "tea_path_dirname");
    define_ffi_string_transform_fn!(
        ensure_path_basename_fn,
        path_basename_fn,
        "tea_path_basename"
    );
    define_ffi_string_transform_fn!(
        ensure_path_extension_fn,
        path_extension_fn,
        "tea_path_extension"
    );

    define_ffi_string2_fn!(
        ensure_path_set_extension_fn,
        path_set_extension_fn,
        "tea_path_set_extension"
    );
    define_ffi_string_transform_fn!(
        ensure_path_strip_extension_fn,
        path_strip_extension_fn,
        "tea_path_strip_extension"
    );
    define_ffi_string_transform_fn!(
        ensure_path_normalize_fn,
        path_normalize_fn,
        "tea_path_normalize"
    );
    define_ffi_string2_fn!(
        ensure_path_relative_fn,
        path_relative_fn,
        "tea_path_relative"
    );
    define_ffi_string_predicate_fn!(
        ensure_path_is_absolute_fn,
        path_is_absolute_fn,
        "tea_path_is_absolute"
    );
    define_ffi_string_getter_fn!(
        ensure_path_separator_fn,
        path_separator_fn,
        "tea_path_separator"
    );

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

    // Filesystem functions
    define_ffi_string_transform_fn!(ensure_fs_read_text_fn, fs_read_text_fn, "tea_fs_read_text");
    define_ffi_string_to_list_fn!(
        ensure_fs_read_bytes_fn,
        fs_read_bytes_fn,
        "tea_fs_read_bytes"
    );
    define_ffi_string_setter_fn!(
        ensure_fs_ensure_dir_fn,
        fs_ensure_dir_fn,
        "tea_fs_ensure_dir"
    );
    define_ffi_string_setter_fn!(
        ensure_fs_ensure_parent_fn,
        fs_ensure_parent_fn,
        "tea_fs_ensure_parent"
    );
    define_ffi_string_setter_fn!(ensure_fs_remove_fn, fs_remove_fn, "tea_fs_remove");
    define_ffi_string_predicate_fn!(ensure_fs_exists_fn, fs_exists_fn, "tea_fs_exists");
    define_ffi_string_predicate_fn!(ensure_fs_is_dir_fn, fs_is_dir_fn, "tea_fs_is_dir");
    define_ffi_string_predicate_fn!(
        ensure_fs_is_symlink_fn,
        fs_is_symlink_fn,
        "tea_fs_is_symlink"
    );
    define_ffi_string_to_list_fn!(ensure_fs_list_dir_fn, fs_list_dir_fn, "tea_fs_list_dir");
    define_ffi_string_to_list_fn!(ensure_fs_walk_fn, fs_walk_fn, "tea_fs_walk");
    define_ffi_string_to_list_fn!(ensure_fs_glob_fn, fs_glob_fn, "tea_fs_glob");
    define_ffi_string_to_int_fn!(ensure_fs_size_fn, fs_size_fn, "tea_fs_size");
    define_ffi_string_to_int_fn!(ensure_fs_modified_fn, fs_modified_fn, "tea_fs_modified");
    define_ffi_string_to_int_fn!(
        ensure_fs_permissions_fn,
        fs_permissions_fn,
        "tea_fs_permissions"
    );

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

    fn ensure_string_concat_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.string_concat_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(
            &[self.string_ptr_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_string_concat", fn_type, Some(Linkage::External));
        self.string_concat_fn = Some(func);
        func
    }

    fn ensure_string_push_str_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.string_push_str_fn {
            return func;
        }
        // tea_string_push_str(target: *TeaString, src: *TeaString) -> *TeaString
        let fn_type = self.string_ptr_type().fn_type(
            &[self.string_ptr_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func =
            self.module
                .add_function("tea_string_push_str", fn_type, Some(Linkage::External));
        self.string_push_str_fn = Some(func);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(
            &[
                self.struct_ptr_type().into(),
                self.int_type().into(),
                ptr_type.into(), // TeaValue pointer
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

    fn ensure_string_index(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.string_index_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(
            &[self.string_ptr_type().into(), self.int_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_string_index", fn_type, Some(Linkage::External));
        self.string_index_fn = Some(func);
        func
    }

    fn ensure_list_concat_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.list_concat_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(
            &[self.list_ptr_type().into(), self.list_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_list_concat", fn_type, Some(Linkage::External));
        self.list_concat_fn = Some(func);
        func
    }

    fn ensure_string_slice(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.string_slice_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(
            &[
                self.string_ptr_type().into(),
                self.int_type().into(),
                self.int_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("tea_string_slice", fn_type, Some(Linkage::External));
        self.string_slice_fn = Some(func);
        func
    }

    fn ensure_list_slice(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.list_slice_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(
            &[
                self.list_ptr_type().into(),
                self.int_type().into(),
                self.int_type().into(),
                self.context.bool_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("tea_list_slice", fn_type, Some(Linkage::External));
        self.list_slice_fn = Some(func);
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
        // Returns void, writes result to output pointer to avoid ARM64 ABI issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(
            &[
                self.struct_ptr_type().into(),
                self.int_type().into(),
                ptr_type.into(), // output pointer for TeaValue
            ],
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(
            &[
                self.error_ptr_type().into(),
                self.int_type().into(),
                ptr_type.into(), // TeaValue pointer
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
        // Returns void, writes result to output pointer to avoid ARM64 ABI issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(
            &[
                self.error_ptr_type().into(),
                self.int_type().into(),
                ptr_type.into(), // output pointer for TeaValue
            ],
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.int_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.float_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.string_ptr_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.list_ptr_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.dict_ptr_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.struct_ptr_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.error_ptr_type().fn_type(&[ptr_type.into()], false);
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
        // Pass TeaValue by pointer to avoid ARM64 ABI struct passing issues
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.closure_ptr_type().fn_type(&[ptr_type.into()], false);
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

    fn ensure_args_program_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.args_program_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_args_program", fn_type, Some(Linkage::External));
        self.args_program_fn = Some(func);
        func
    }

    // Regex ensure functions
    fn ensure_regex_compile_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_compile_fn {
            return func;
        }
        let fn_type = self
            .int_type()
            .fn_type(&[self.string_ptr_type().into()], false);
        let func = self
            .module
            .add_function("tea_regex_compile", fn_type, Some(Linkage::External));
        self.regex_compile_fn = Some(func);
        func
    }

    fn ensure_regex_is_match_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_is_match_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[self.int_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_regex_is_match", fn_type, Some(Linkage::External));
        self.regex_is_match_fn = Some(func);
        func
    }

    fn ensure_regex_find_all_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_find_all_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(
            &[self.int_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_regex_find_all", fn_type, Some(Linkage::External));
        self.regex_find_all_fn = Some(func);
        func
    }

    fn ensure_regex_captures_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_captures_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(
            &[self.int_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_regex_captures", fn_type, Some(Linkage::External));
        self.regex_captures_fn = Some(func);
        func
    }

    fn ensure_regex_replace_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_replace_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(
            &[
                self.int_type().into(),
                self.string_ptr_type().into(),
                self.string_ptr_type().into(),
            ],
            false,
        );
        let func = self
            .module
            .add_function("tea_regex_replace", fn_type, Some(Linkage::External));
        self.regex_replace_fn = Some(func);
        func
    }

    fn ensure_regex_replace_all_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_replace_all_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(
            &[
                self.int_type().into(),
                self.string_ptr_type().into(),
                self.string_ptr_type().into(),
            ],
            false,
        );
        let func =
            self.module
                .add_function("tea_regex_replace_all", fn_type, Some(Linkage::External));
        self.regex_replace_all_fn = Some(func);
        func
    }

    fn ensure_regex_split_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.regex_split_fn {
            return func;
        }
        let fn_type = self.list_ptr_type().fn_type(
            &[self.int_type().into(), self.string_ptr_type().into()],
            false,
        );
        let func = self
            .module
            .add_function("tea_regex_split", fn_type, Some(Linkage::External));
        self.regex_split_fn = Some(func);
        func
    }

    fn ensure_read_line_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.read_line_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_read_line", fn_type, Some(Linkage::External));
        self.read_line_fn = Some(func);
        func
    }

    fn ensure_read_all_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.read_all_fn {
            return func;
        }
        let fn_type = self.string_ptr_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_read_all", fn_type, Some(Linkage::External));
        self.read_all_fn = Some(func);
        func
    }

    fn ensure_is_tty_fn(&mut self) -> FunctionValue<'ctx> {
        if let Some(func) = self.is_tty_fn {
            return func;
        }
        let fn_type = self.context.i32_type().fn_type(&[], false);
        let func = self
            .module
            .add_function("tea_is_tty", fn_type, Some(Linkage::External));
        self.is_tty_fn = Some(func);
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
        // Pass TeaValue by pointer for ARM64 ABI compatibility
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let param_types = [
            self.struct_template_ptr_type().into(),
            self.string_ptr_type().into(),
            ptr_type.into(), // args_ptr
            ptr_type.into(), // env_ptr
            ptr_type.into(), // cwd_ptr
            ptr_type.into(), // stdin_ptr
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
        // Pass TeaValue by pointer for ARM64 ABI compatibility
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let param_types = [
            self.string_ptr_type().into(),
            ptr_type.into(), // args_ptr
            ptr_type.into(), // env_ptr
            ptr_type.into(), // cwd_ptr
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
