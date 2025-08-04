use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
#[cfg(feature = "llvm-aot")]
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use clap::{ArgAction, Parser, ValueEnum};
use tea_compiler::{
    format_source, CompileOptions, Compiler, Diagnostic, DiagnosticLevel, SourceFile, SourceId,
    TestRunOptions, Vm,
};

#[cfg(feature = "llvm-aot")]
use tea_compiler::aot::{self, ObjectCompileOptions};

#[cfg(feature = "llvm-aot")]
use flate2::{write::GzEncoder, Compression};
#[cfg(feature = "llvm-aot")]
use hmac::{Hmac, Mac};
#[cfg(feature = "llvm-aot")]
use serde_json::json;
#[cfg(feature = "llvm-aot")]
use sha2::{Digest, Sha256};
#[cfg(feature = "llvm-aot")]
use tar::{Builder, Header, HeaderMode};
#[cfg(feature = "llvm-aot")]
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[cfg(feature = "llvm-aot")]
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Emit {
    Ast,
    Bytecode,
    LlvmIr,
    #[cfg(feature = "llvm-aot")]
    Obj,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum Backend {
    #[default]
    Bytecode,
    #[cfg(feature = "llvm-aot")]
    Llvm,
}

impl Backend {
    #[cfg(feature = "llvm-aot")]
    const fn build_default() -> Self {
        Backend::Llvm
    }

    #[cfg(not(feature = "llvm-aot"))]
    const fn build_default() -> Self {
        Backend::Bytecode
    }
}

#[derive(Parser)]
#[command(name = "tea", version)]
struct RunCli {
    /// Path to a tea-lang source file.
    input: PathBuf,

    /// Dump the token stream produced by the lexer.
    #[arg(long)]
    dump_tokens: bool,

    /// Emit additional compiler output (e.g. `ast`, `bytecode`).
    #[arg(long, value_enum)]
    emit: Vec<Emit>,

    /// Skip executing the compiled program.
    #[arg(long)]
    no_run: bool,

    /// Select the execution backend (`bytecode` or `llvm`).
    #[arg(long, value_enum, default_value_t = Backend::default())]
    backend: Backend,

    /// Arguments forwarded to the tea script.
    #[arg(value_name = "ARG", trailing_var_arg = true, num_args = 0..)]
    script_args: Vec<String>,
}

#[derive(Parser)]
#[command(
    name = "tea build",
    version,
    about = "Compile a tea-lang source file to a native executable."
)]
struct BuildCli {
    /// Path to a tea-lang source file.
    input: PathBuf,

    /// Destination for the produced binary (defaults to `bin/<name>`).
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Backend used for code generation (only `llvm` is supported).
    #[arg(long, value_enum, default_value_t = Backend::build_default())]
    backend: Backend,

    /// Emit additional compiler output alongside the executable.
    #[arg(long, value_enum)]
    emit: Vec<Emit>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "TRIPLE")]
    target: Option<String>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "CPU")]
    cpu: Option<String>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "FEATURES")]
    features: Option<String>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "LEVEL")]
    opt_level: Option<String>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, action = ArgAction::SetTrue)]
    bundle: bool,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "PATH", requires = "bundle")]
    bundle_output: Option<PathBuf>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, action = ArgAction::SetTrue)]
    checksum: bool,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "PATH", requires = "checksum")]
    checksum_output: Option<PathBuf>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "PATH")]
    signature_key: Option<PathBuf>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "PATH", requires = "signature_key")]
    signature_output: Option<PathBuf>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "PATH")]
    rustc: Option<PathBuf>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long, value_name = "PATH")]
    linker: Option<PathBuf>,

    #[cfg(feature = "llvm-aot")]
    #[arg(long = "linker-arg", value_name = "ARG")]
    linker_args: Vec<String>,

    #[cfg(not(feature = "llvm-aot"))]
    #[arg(long, hide = true)]
    _target: Option<String>,

    #[cfg(not(feature = "llvm-aot"))]
    #[arg(long, hide = true)]
    _unused: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "tea fmt",
    version,
    about = "Format tea-lang source files in-place."
)]
struct FmtCli {
    /// Paths to format.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Do not write files; exit with an error if changes are needed.
    #[arg(long)]
    check: bool,
}

#[derive(Parser)]
#[command(
    name = "tea test",
    version,
    about = "Discover and run tea-lang test blocks."
)]
struct TestCli {
    /// Paths or directories containing tea test files (defaults to `tests/`).
    #[arg(value_name = "PATH")]
    inputs: Vec<PathBuf>,

    /// List discovered tests without executing them.
    #[arg(long)]
    list: bool,

    /// Only run tests whose names contain this substring (case-insensitive).
    #[arg(long, value_name = "FILTER")]
    filter: Option<String>,

    /// Stop after the first test failure.
    #[arg(long)]
    fail_fast: bool,

    /// Update stored snapshots instead of comparing them.
    #[arg(long)]
    update_snapshots: bool,
}

fn main() -> Result<()> {
    let raw: Vec<OsString> = std::env::args_os().collect();
    if raw.get(1).map(|arg| arg == "build").unwrap_or(false) {
        return handle_build(raw);
    }
    if raw.get(1).map(|arg| arg == "fmt").unwrap_or(false) {
        return handle_fmt(raw);
    }
    if raw.get(1).map(|arg| arg == "test").unwrap_or(false) {
        return handle_test(raw);
    }

    let run_cli = RunCli::parse_from(raw);
    run_program(run_cli)
}

fn handle_build(raw: Vec<OsString>) -> Result<()> {
    let mut args = raw.clone();
    if !args.is_empty() {
        args.remove(1); // drop the literal "build"
    }

    #[cfg(feature = "llvm-aot")]
    {
        let build_cli = BuildCli::parse_from(args);
        run_build(build_cli)
    }

    #[cfg(not(feature = "llvm-aot"))]
    {
        let _ = BuildCli::parse_from(args);
        bail!("LLVM backend not enabled; rebuild with `--features tea-cli/llvm-aot`");
    }
}

fn handle_fmt(raw: Vec<OsString>) -> Result<()> {
    let mut args = raw.clone();
    if !args.is_empty() {
        args.remove(1); // drop the literal "fmt"
    }
    let cli = FmtCli::parse_from(args);
    run_fmt(&cli)
}

fn handle_test(raw: Vec<OsString>) -> Result<()> {
    let mut args = raw.clone();
    if !args.is_empty() {
        args.remove(1); // drop the literal "test"
    }
    let cli = TestCli::parse_from(args);
    run_test(&cli)
}

fn run_fmt(cli: &FmtCli) -> Result<()> {
    let mut had_changes = false;
    let mut targets = BTreeSet::new();

    for input in &cli.inputs {
        collect_tea_files(input, &mut targets)?;
    }

    for path in &targets {
        let contents =
            fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;
        let formatted = format_source(&contents);

        if contents == formatted {
            continue;
        }

        if cli.check {
            had_changes = true;
            eprintln!("needs formatting: {}", path.display());
        } else {
            fs::write(path, formatted).with_context(|| format!("Failed to write {:?}", path))?;
            println!("Formatted {}", path.display());
        }
    }

    if cli.check && had_changes {
        bail!("one or more files require formatting");
    }

    Ok(())
}

fn run_test(cli: &TestCli) -> Result<()> {
    let workspace_root = detect_workspace_root()?;

    let target_paths = if cli.inputs.is_empty() {
        let default = workspace_root.join("tests");
        if default.exists() {
            vec![default]
        } else {
            bail!("tests/ directory not found; pass explicit paths to `tea test`");
        }
    } else {
        cli.inputs
            .iter()
            .map(|path| {
                if path.is_relative() {
                    workspace_root.join(path)
                } else {
                    path.clone()
                }
            })
            .collect()
    };

    let mut files = BTreeSet::new();
    for path in &target_paths {
        collect_tea_files(path, &mut files)?;
    }

    if files.is_empty() {
        println!("no test files found");
        return Ok(());
    }

    let filter = cli.filter.as_ref().map(|f| f.to_ascii_lowercase());
    let filter_ref = filter.as_deref();
    let mut total = 0usize;
    let mut failed = 0usize;
    let mut found_any = false;

    let snapshots_root = workspace_root.join("tests/__snapshots__");

    'files: for path in files {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let source = SourceFile::new(SourceId(0), path.clone(), contents);
        let line_cache: Vec<&str> = source.contents.lines().collect();

        let mut compiler = Compiler::new(CompileOptions::default());
        let compilation = compiler
            .compile(&source)
            .with_context(|| format!("failed to compile {}", path.display()))?;

        if !compiler.diagnostics().is_empty() {
            eprintln!("Diagnostics for {}:", path.display());
            for diagnostic in compiler.diagnostics().entries() {
                print_diagnostic(&source, &line_cache, diagnostic);
            }
            bail!("compilation failed for {}", path.display());
        }

        let matching_tests: Vec<_> = compilation
            .program
            .tests
            .iter()
            .filter(|test| test_matches(&test.name, filter_ref))
            .cloned()
            .collect();

        if matching_tests.is_empty() {
            continue;
        }

        found_any = true;
        let display_path = path
            .strip_prefix(&workspace_root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let relative_snapshot_path = relative_snapshot_path(&workspace_root, &path);
        let run_options = TestRunOptions {
            update_snapshots: cli.update_snapshots,
            snapshot_root: snapshots_root.clone(),
            relative_test_path: relative_snapshot_path,
        };

        if cli.list {
            println!("{}", display_path);
            for test in matching_tests {
                println!("  {}", test.name);
            }
            continue;
        }

        let mut vm = Vm::new(&compilation.program);
        let outcomes = vm.run_tests(filter_ref, Some(&run_options))?;
        if outcomes.is_empty() {
            continue;
        }

        println!("{}", display_path);
        for outcome in outcomes {
            total += 1;
            match outcome.status {
                tea_compiler::TestStatus::Passed => {
                    println!("  PASS {}", outcome.name);
                }
                tea_compiler::TestStatus::Failed { message } => {
                    failed += 1;
                    println!("  FAIL {}", outcome.name);
                    if !message.is_empty() {
                        println!("       {message}");
                    }
                    if outcome.span.line > 0 {
                        println!("       at {}:{}", display_path, outcome.span.line);
                    }
                    if cli.fail_fast {
                        break 'files;
                    }
                }
            }
        }
    }

    if cli.list {
        if !found_any {
            println!("no tests discovered");
        }
        return Ok(());
    }

    if !found_any {
        println!("no tests matched the current selection");
        return Ok(());
    }

    let passed = total.saturating_sub(failed);
    println!("\nSummary: {passed} passed, {failed} failed, {total} total");

    if failed > 0 {
        bail!("tests failed");
    }

    Ok(())
}

fn relative_snapshot_path(workspace_root: &Path, test_path: &Path) -> PathBuf {
    let relative = test_path.strip_prefix(workspace_root).unwrap_or(test_path);
    if let Ok(stripped) = relative.strip_prefix(Path::new("tests")) {
        if stripped.as_os_str().is_empty() {
            relative.to_path_buf()
        } else {
            stripped.to_path_buf()
        }
    } else {
        relative.to_path_buf()
    }
}

fn collect_tea_files(path: &PathBuf, targets: &mut BTreeSet<PathBuf>) -> Result<()> {
    let metadata = fs::metadata(path).with_context(|| format!("Failed to access {:?}", path))?;

    if metadata.is_dir() {
        let mut child_paths = Vec::new();
        for entry in
            fs::read_dir(path).with_context(|| format!("Failed to read directory {:?}", path))?
        {
            let entry =
                entry.with_context(|| format!("Failed to access entry within {:?}", path))?;
            child_paths.push(entry.path());
        }
        child_paths.sort();
        for child in child_paths {
            collect_tea_files(&child, targets)?;
        }
    } else if metadata.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("tea") {
            targets.insert(path.clone());
        }
    }

    Ok(())
}

fn detect_workspace_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .ok_or_else(|| anyhow!("failed to locate workspace root from manifest dir"))?;
    Ok(root.to_path_buf())
}

fn test_matches(name: &str, filter: Option<&str>) -> bool {
    if let Some(needle) = filter {
        name.to_ascii_lowercase().contains(needle)
    } else {
        true
    }
}

fn run_program(cli: RunCli) -> Result<()> {
    let contents = fs::read_to_string(&cli.input)
        .with_context(|| format!("Failed to read {:?}", cli.input))?;

    let source = SourceFile::new(SourceId(0), cli.input.clone(), contents);
    let line_cache: Vec<&str> = source.contents.lines().collect();
    let mut compiler = Compiler::new(CompileOptions {
        dump_tokens: cli.dump_tokens,
    });

    let compilation = match compiler.compile(&source) {
        Ok(comp) => comp,
        Err(err) => {
            if !compiler.diagnostics().is_empty() {
                eprintln!("Diagnostics:");
                for diagnostic in compiler.diagnostics().entries() {
                    print_diagnostic(&source, &line_cache, diagnostic);
                }
            }
            return Err(err.context("Compilation failed"));
        }
    };

    if cli.emit.contains(&Emit::Ast) {
        println!("{:#?}", compilation.module);
    }

    if cli.emit.contains(&Emit::Bytecode) {
        for (index, instruction) in compilation.program.chunk.instructions.iter().enumerate() {
            println!("{index:04}: {instruction}");
        }
    }

    if cli.emit.contains(&Emit::LlvmIr) {
        #[cfg(feature = "llvm-aot")]
        {
            let ir = aot::compile_module_to_llvm_ir(&compilation.module)?;
            println!("{ir}");
        }
        #[cfg(not(feature = "llvm-aot"))]
        {
            bail!("LLVM backend not enabled; rebuild with `--features tea-cli/llvm-aot`");
        }
    }

    #[cfg(feature = "llvm-aot")]
    if cli.emit.contains(&Emit::Obj) {
        let object_path = object_output_for_source(&cli.input);
        if let Some(parent) = object_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        aot::compile_module_to_object(
            &compilation.module,
            &object_path,
            &ObjectCompileOptions::default(),
        )?;
        println!("{}", object_path.display());
    }

    if !compiler.diagnostics().is_empty() {
        eprintln!("Diagnostics:");
        for diagnostic in compiler.diagnostics().entries() {
            print_diagnostic(&source, &line_cache, diagnostic);
        }
    }

    let program_label = cli
        .input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
        .unwrap_or_else(|| cli.input.display().to_string());

    #[cfg(feature = "llvm-aot")]
    {
        match cli.backend {
            Backend::Bytecode => {
                if !cli.no_run {
                    let mut vm = Vm::new(&compilation.program);
                    vm.set_cli_context(program_label.clone(), cli.script_args.clone());
                    vm.run().context("Runtime error")?;
                }
            }
            Backend::Llvm => {
                bail!("Executing via the LLVM backend is not supported yet; use `tea build` to produce a binary.");
            }
        }
    }

    #[cfg(not(feature = "llvm-aot"))]
    {
        match cli.backend {
            Backend::Bytecode => {
                if !cli.no_run {
                    let mut vm = Vm::new(&compilation.program);
                    vm.set_cli_context(program_label.clone(), cli.script_args.clone());
                    vm.run().context("Runtime error")?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(feature = "llvm-aot")]
fn run_build(cli: BuildCli) -> Result<()> {
    let contents = fs::read_to_string(&cli.input)
        .with_context(|| format!("Failed to read {:?}", cli.input))?;

    let source = SourceFile::new(SourceId(0), cli.input.clone(), contents);
    let line_cache: Vec<&str> = source.contents.lines().collect();
    let mut compiler = Compiler::new(CompileOptions::default());

    let compilation = match compiler.compile(&source) {
        Ok(comp) => comp,
        Err(err) => {
            if !compiler.diagnostics().is_empty() {
                eprintln!("Diagnostics:");
                for diagnostic in compiler.diagnostics().entries() {
                    print_diagnostic(&source, &line_cache, diagnostic);
                }
            }
            return Err(err.context("Compilation failed"));
        }
    };

    if cli.emit.contains(&Emit::Ast) {
        println!("{:#?}", compilation.module);
    }

    if cli.emit.contains(&Emit::Bytecode) {
        for (index, instruction) in compilation.program.chunk.instructions.iter().enumerate() {
            println!("{index:04}: {instruction}");
        }
    }

    if !compiler.diagnostics().is_empty() {
        eprintln!("Diagnostics:");
        for diagnostic in compiler.diagnostics().entries() {
            print_diagnostic(&source, &line_cache, diagnostic);
        }
    }

    match cli.backend {
        Backend::Bytecode => {
            bail!("the bytecode backend cannot produce native executables; pass `--backend llvm`")
        }
        Backend::Llvm => build_with_llvm(&cli, &compilation),
    }
}

#[cfg(feature = "llvm-aot")]
fn build_with_llvm(cli: &BuildCli, compilation: &tea_compiler::Compilation) -> Result<()> {
    let mut output = match &cli.output {
        Some(path) => path.clone(),
        None => default_binary_path(&cli.input),
    };
    if cfg!(windows) && output.extension().is_none() {
        output.set_extension("exe");
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut object_options = ObjectCompileOptions::default();
    object_options.entry_symbol = Some("tea_main");
    object_options.triple = cli.target.as_deref();
    if let Some(cpu) = cli.cpu.as_deref() {
        object_options.cpu = Some(cpu);
    }
    if let Some(features) = cli.features.as_deref() {
        object_options.features = Some(features);
    }
    if let Some(level) = cli.opt_level.as_deref() {
        object_options.opt_level = parse_opt_level(level)?;
    }

    let object_path = object_path_for_output(&output);
    if let Err(err) =
        aot::compile_module_to_object(&compilation.module, &object_path, &object_options)
    {
        if err
            .to_string()
            .contains("No available targets are compatible")
        {
            bail!(
                "{}

Install an LLVM toolchain with support for {} or re-run with `--target <triple>`.",
                err,
                object_options.triple.unwrap_or("the host target")
            );
        } else {
            return Err(err);
        }
    }

    if cli.emit.contains(&Emit::LlvmIr) {
        let ir = aot::compile_module_to_llvm_ir(&compilation.module)?;
        println!("{ir}");
    }

    if cli.emit.contains(&Emit::Obj) {
        println!("Object file written to {}", object_path.display());
    }

    let stub_path = object_path.with_extension("stub.rs");
    fs::write(&stub_path, STUB_SOURCE)?;

    let runtime_rlib = locate_runtime_rlib(current_profile())?;
    let rustc_path = cli
        .rustc
        .clone()
        .or_else(|| std::env::var_os("RUSTC").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("rustc"));
    let rustc_info = detect_rustc_info(&rustc_path);

    link_with_rustc(
        &rustc_path,
        &stub_path,
        &object_path,
        &runtime_rlib,
        &output,
        cli.target.as_deref(),
        cli.linker.as_deref(),
        &cli.linker_args,
    )?;

    if !cli.emit.contains(&Emit::Obj) {
        let _ = fs::remove_file(&object_path);
    }
    let _ = fs::remove_file(&stub_path);

    let build_time = build_timestamp()?;
    let sha256 = compute_sha256(&output)?;

    if cli.checksum {
        let checksum_path = cli
            .checksum_output
            .clone()
            .unwrap_or_else(|| checksum_path_for(&output));
        write_checksum_file(&checksum_path, &output, &sha256)?;
        println!("Checksum written to {}", checksum_path.display());
    }

    if let Some(key_path) = &cli.signature_key {
        let key_bytes = fs::read(key_path)
            .with_context(|| format!("failed to read signature key at {:?}", key_path))?;
        let signature_path = cli
            .signature_output
            .clone()
            .unwrap_or_else(|| signature_path_for(&output));
        write_signature_file(&signature_path, &output, &key_bytes)?;
        println!("Signature written to {}", signature_path.display());
    }

    let target_label = cli
        .target
        .as_deref()
        .map(aot::normalize_target_triple)
        .or_else(|| rustc_info.host.as_deref().map(aot::normalize_target_triple))
        .unwrap_or_else(|| "unknown".to_string());
    let opt_level_label = cli
        .opt_level
        .clone()
        .unwrap_or_else(|| opt_level_to_string(object_options.opt_level));

    if cli.bundle {
        let bundle_path = cli
            .bundle_output
            .clone()
            .unwrap_or_else(|| default_bundle_path(&output));
        let metadata_json = build_metadata_json(
            &output,
            &cli.input,
            &target_label,
            cli.cpu.as_deref(),
            cli.features.as_deref(),
            &opt_level_label,
            &build_time.iso,
            &sha256,
            rustc_info.version.as_deref(),
        )?;
        bundle_artifacts(
            &output,
            &bundle_path,
            &metadata_json,
            &sha256,
            build_time.epoch,
        )?;
        println!("Bundle written to {}", bundle_path.display());
    }

    println!("Built {}", output.display());
    Ok(())
}

fn print_diagnostic(source: &SourceFile, lines: &[&str], diagnostic: &Diagnostic) {
    let (level_label, level_color) = match diagnostic.level {
        DiagnosticLevel::Error => ("error", "  -"),
        DiagnosticLevel::Warning => ("warning", "  ~"),
    };
    eprintln!("{} {}: {}", level_color, level_label, diagnostic.message);
    if let Some(span) = diagnostic.span {
        let display_path = source.path.display();
        eprintln!("     --> {}:{}:{}", display_path, span.line, span.column);

        if let Some(raw_line) = lines.get(span.line.saturating_sub(1)) {
            let display_line = raw_line.replace('\t', "    ");
            eprintln!("      {}", display_line);

            let mut caret_line = String::from("      ");
            let mut current_col = 1usize;
            for ch in raw_line.chars() {
                if current_col >= span.column {
                    break;
                }
                match ch {
                    '\t' => caret_line.push_str("    "),
                    _ => caret_line.push(' '),
                }
                current_col += 1;
            }

            let highlight_len = if span.end_line == span.line {
                span.end_column
                    .saturating_sub(span.column)
                    .saturating_add(1)
            } else {
                display_line.chars().count().saturating_sub(
                    span.column
                        .saturating_sub(1)
                        .min(display_line.chars().count()),
                )
            };

            caret_line.push_str(&"^".repeat(highlight_len.max(1)));
            eprintln!("{}", caret_line);
        }
    }
}

#[cfg(feature = "llvm-aot")]
fn default_binary_path(input: &Path) -> PathBuf {
    let mut dir = PathBuf::from("bin");
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    dir.push(stem);
    dir
}

#[cfg(feature = "llvm-aot")]
fn object_extension() -> &'static str {
    if cfg!(windows) {
        "obj"
    } else {
        "o"
    }
}

#[cfg(feature = "llvm-aot")]
fn object_path_for_output(output: &Path) -> PathBuf {
    let mut path = output.to_owned();
    path.set_extension(object_extension());
    path
}

#[cfg(feature = "llvm-aot")]
fn object_output_for_source(source: &Path) -> PathBuf {
    let mut path = source.to_owned();
    path.set_extension(object_extension());
    path
}

#[cfg(feature = "llvm-aot")]
fn current_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[cfg(feature = "llvm-aot")]
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[cfg(feature = "llvm-aot")]
fn locate_runtime_rlib(profile: &str) -> Result<PathBuf> {
    if let Ok(path) = std::env::var("TEA_RUNTIME_RLIB") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let target_dir = std::env::var("TEA_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root().join("target"));

    let mut search_dirs = Vec::new();
    search_dirs.push(target_dir.join(profile).join("deps"));

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in
            fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("rlib") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains("tea_runtime") {
                        return Ok(path);
                    }
                }
            }
        }
    }

    bail!(
        "unable to locate tea-runtime rlib; run `cargo build -p tea-runtime` to build the runtime archive"
    );
}

#[cfg(feature = "llvm-aot")]
fn link_with_rustc(
    rustc: &Path,
    stub_path: &Path,
    object_path: &Path,
    runtime_rlib: &Path,
    output: &Path,
    target: Option<&str>,
    linker: Option<&Path>,
    linker_args: &[String],
) -> Result<()> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let deps_dir = runtime_rlib
        .parent()
        .ok_or_else(|| anyhow::anyhow!("failed to resolve runtime dependency directory"))?;

    let crate_name = output
        .file_stem()
        .and_then(|name| name.to_str())
        .map(|s| {
            s.chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "tea_build".to_string());

    let mut cmd = Command::new(rustc);
    cmd.arg(stub_path);
    cmd.arg("--crate-type").arg("bin");
    cmd.arg("--crate-name").arg(&crate_name);
    cmd.arg("--extern")
        .arg(format!("tea_runtime={}", runtime_rlib.display()));
    cmd.arg("-L")
        .arg(format!("dependency={}", deps_dir.display()));
    cmd.arg("--edition").arg("2021");
    cmd.arg(format!("-Clink-arg={}", object_path.to_string_lossy()));
    cmd.arg("-o").arg(output);
    if let Some(target) = target {
        cmd.arg("--target").arg(target);
    }
    if let Some(linker) = linker {
        cmd.arg(format!("-Clinker={}", linker.display()));
    }
    for arg in linker_args {
        cmd.arg(format!("-Clink-arg={arg}"));
    }

    let output_status = match cmd.output() {
        Ok(status) => status,
        Err(error) => {
            if error.kind() == io::ErrorKind::NotFound {
                bail!(
                    "failed to invoke rustc linker at {}: command not found",
                    rustc.display()
                );
            } else {
                return Err(error).context("failed to invoke rustc linker");
            }
        }
    };
    if !output_status.status.success() {
        let stderr = String::from_utf8_lossy(&output_status.stderr);
        let stdout = String::from_utf8_lossy(&output_status.stdout);
        bail!(
            "linker failed with status {}:
{}
{}",
            output_status.status,
            stdout.trim_end(),
            stderr.trim_end()
        );
    }

    Ok(())
}

#[cfg(feature = "llvm-aot")]
const STUB_SOURCE: &str = r#"extern crate tea_runtime;

extern "C" {
    fn tea_main() -> i32;
}

fn main() {
    std::process::exit(unsafe { tea_main() });
}
"#;
#[cfg(feature = "llvm-aot")]
#[derive(Default)]
struct RustcInfo {
    version: Option<String>,
    host: Option<String>,
}

#[cfg(feature = "llvm-aot")]
struct BuildTimestamp {
    iso: String,
    epoch: u64,
}

#[cfg(feature = "llvm-aot")]
fn parse_opt_level(level: &str) -> Result<tea_compiler::aot::OptimizationLevel> {
    use tea_compiler::aot::OptimizationLevel;
    match level {
        "0" => Ok(OptimizationLevel::None),
        "1" => Ok(OptimizationLevel::Less),
        "2" => Ok(OptimizationLevel::Default),
        "3" => Ok(OptimizationLevel::Aggressive),
        "s" | "z" => Ok(OptimizationLevel::Default),
        other => bail!("unsupported opt-level '{other}'; use 0,1,2,3,s, or z"),
    }
}

#[cfg(feature = "llvm-aot")]
fn opt_level_to_string(level: tea_compiler::aot::OptimizationLevel) -> String {
    use tea_compiler::aot::OptimizationLevel;
    match level {
        OptimizationLevel::None => "0",
        OptimizationLevel::Less => "1",
        OptimizationLevel::Default => "2",
        OptimizationLevel::Aggressive => "3",
    }
    .to_string()
}

#[cfg(feature = "llvm-aot")]
fn build_timestamp() -> Result<BuildTimestamp> {
    let epoch = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) => value
            .parse::<i64>()
            .with_context(|| format!("invalid SOURCE_DATE_EPOCH value: {}", value))?,
        Err(_) => OffsetDateTime::now_utc().unix_timestamp(),
    };

    let datetime =
        OffsetDateTime::from_unix_timestamp(epoch).unwrap_or_else(|_| OffsetDateTime::now_utc());
    let iso = datetime
        .format(&Rfc3339)
        .context("failed to format build timestamp")?;
    Ok(BuildTimestamp {
        iso,
        epoch: epoch.max(0) as u64,
    })
}

#[cfg(feature = "llvm-aot")]
fn compute_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .with_context(|| format!("failed to open {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{:02x}", byte)).collect())
}

#[cfg(feature = "llvm-aot")]
fn checksum_path_for(binary: &Path) -> PathBuf {
    let mut path = binary.to_owned();
    path.set_extension("sha256");
    path
}

#[cfg(feature = "llvm-aot")]
fn signature_path_for(binary: &Path) -> PathBuf {
    let mut path = binary.to_owned();
    path.set_extension("sig");
    path
}

#[cfg(feature = "llvm-aot")]
fn default_bundle_path(binary: &Path) -> PathBuf {
    let mut path = binary.to_owned();
    path.set_extension("tar.gz");
    path
}

#[cfg(feature = "llvm-aot")]
fn write_checksum_file(path: &Path, artifact: &Path, hash: &str) -> Result<()> {
    let name = artifact
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("artifact");
    let content = format!("{hash}  {name}\n");
    fs::write(path, content.as_bytes())
        .with_context(|| format!("failed to write checksum to {}", path.display()))
}

#[cfg(feature = "llvm-aot")]
fn write_signature_file(path: &Path, artifact: &Path, key: &[u8]) -> Result<()> {
    if key.is_empty() {
        bail!("signature key is empty");
    }
    let mut file = File::open(artifact)
        .with_context(|| format!("failed to open {} for signing", artifact.display()))?;
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| anyhow::anyhow!("invalid signature key length"))?;
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        mac.update(&buffer[..read]);
    }
    let signature = mac.finalize().into_bytes();
    let mut hex = String::with_capacity(signature.len() * 2 + 1);
    for byte in signature {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex.push('\n');
    fs::write(path, hex.as_bytes())
        .with_context(|| format!("failed to write signature to {}", path.display()))
}

#[cfg(feature = "llvm-aot")]
fn bundle_artifacts(
    binary: &Path,
    bundle_path: &Path,
    metadata_json: &str,
    sha256: &str,
    mtime: u64,
) -> Result<()> {
    if let Some(parent) = bundle_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }

    let file = File::create(bundle_path)
        .with_context(|| format!("failed to create bundle at {}", bundle_path.display()))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    builder.mode(HeaderMode::Deterministic);

    let mut binary_header = Header::new_gnu();
    let binary_name = binary
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("binary");
    let binary_size = fs::metadata(binary)?.len();
    binary_header.set_path(binary_name)?;
    binary_header.set_size(binary_size);
    binary_header.set_mode(0o755);
    binary_header.set_mtime(mtime);
    binary_header.set_uid(0);
    binary_header.set_gid(0);
    binary_header.set_cksum();
    let mut binary_file = File::open(binary)?;
    builder.append(&binary_header, &mut binary_file)?;

    let metadata_bytes = metadata_json.as_bytes();
    let mut metadata_header = Header::new_gnu();
    metadata_header.set_path("build.json")?;
    metadata_header.set_size(metadata_bytes.len() as u64);
    metadata_header.set_mode(0o644);
    metadata_header.set_mtime(mtime);
    metadata_header.set_uid(0);
    metadata_header.set_gid(0);
    metadata_header.set_cksum();
    let mut metadata_cursor = Cursor::new(metadata_bytes);
    builder.append(&metadata_header, &mut metadata_cursor)?;

    let checksum_entry = format!("{sha256}  {binary_name}\n");
    let checksum_bytes = checksum_entry.as_bytes();
    let mut checksum_header = Header::new_gnu();
    checksum_header.set_path("SHA256SUMS")?;
    checksum_header.set_size(checksum_bytes.len() as u64);
    checksum_header.set_mode(0o644);
    checksum_header.set_mtime(mtime);
    checksum_header.set_uid(0);
    checksum_header.set_gid(0);
    checksum_header.set_cksum();
    let mut checksum_cursor = Cursor::new(checksum_bytes);
    builder.append(&checksum_header, &mut checksum_cursor)?;

    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

#[cfg(feature = "llvm-aot")]
fn build_metadata_json(
    binary: &Path,
    source: &Path,
    target: &str,
    cpu: Option<&str>,
    features: Option<&str>,
    opt_level: &str,
    built_at: &str,
    sha256: &str,
    toolchain: Option<&str>,
) -> Result<String> {
    let binary_name = binary
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("binary");
    let metadata = json!({
        "binary": binary_name,
        "source": source.display().to_string(),
        "target": target,
        "cpu": cpu,
        "features": features,
        "opt_level": opt_level,
        "built_at": built_at,
        "sha256": sha256,
        "toolchain": toolchain,
    });
    serde_json::to_string_pretty(&metadata).map_err(Into::into)
}

#[cfg(feature = "llvm-aot")]
fn detect_rustc_info(rustc: &Path) -> RustcInfo {
    let mut info = RustcInfo::default();
    if let Ok(output) = Command::new(rustc).arg("--version").output() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            info.version = Some(version);
        }
    }
    if let Ok(output) = Command::new(rustc)
        .arg("--version")
        .arg("--verbose")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("host:") {
                let host = rest.trim();
                if !host.is_empty() {
                    info.host = Some(host.to_string());
                }
            }
        }
    }
    info
}
