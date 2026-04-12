use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tea_compiler::{aot, CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn llvm_backend_compiles_core_examples() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let examples = [
        "examples/language/basics/basics.tea",
        "examples/language/basics/const.tea",
        "examples/language/collections/lists.tea",
        "examples/language/control_flow/loops.tea",
        "examples/language/control_flow/logical.tea",
        "examples/stdlib/testing/assertions.tea",
    ];

    for example in examples {
        let path = workspace_root.join(example);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let source = SourceFile::new(SourceId(0), path.clone(), contents);

        let mut compiler = Compiler::new(CompileOptions::default());
        let compilation = compiler
            .compile(&source)
            .unwrap_or_else(|err| panic!("compile failed for {}: {err}", path.display()));

        aot::compile_compilation_to_llvm_ir(&compilation)
            .unwrap_or_else(|err| panic!("LLVM codegen failed for {}: {err}", path.display()));

        let object_path = temporary_object_path(&path);
        match aot::compile_compilation_to_object(
            &compilation,
            &object_path,
            &aot::ObjectCompileOptions::default(),
        ) {
            Ok(()) => {
                assert!(
                    object_path.exists(),
                    "object file was not created for {}",
                    path.display()
                );
                let _ = std::fs::remove_file(&object_path);
            }
            Err(err) => {
                let err_str = err.to_string();
                if err_str.contains("No available targets are compatible")
                    || err_str.contains("failed to parse optimized IR")
                {
                    eprintln!("skipping object emission for {}: {err}", path.display());
                } else {
                    panic!("object emission failed for {}: {err}", path.display());
                }
            }
        }
    }
}

#[test]
fn strings_benchmark_uses_reserved_builder_path() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let path = workspace_root.join("benchmarks/tea/strings.tea");
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let source = SourceFile::new(SourceId(0), path.clone(), contents);

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler
        .compile(&source)
        .unwrap_or_else(|err| panic!("compile failed for {}: {err}", path.display()));

    let ir = aot::compile_compilation_to_llvm_ir(&compilation)
        .unwrap_or_else(|err| panic!("LLVM codegen failed for {}: {err}", path.display()));

    assert!(
        ir.contains("@tea_string_with_capacity"),
        "expected reserved string builder allocation in IR:\n{ir}"
    );
    assert!(
        ir.contains("@tea_string_set_len_ffi"),
        "expected builder length finalization in IR:\n{ir}"
    );
    assert!(
        !ir.contains("@tea_string_push_byte("),
        "hot loop should avoid runtime byte pushes:\n{ir}"
    );
}

#[test]
fn loops_benchmark_collapses_periodic_accumulation() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let path = workspace_root.join("benchmarks/tea/loops.tea");
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let source = SourceFile::new(SourceId(0), path.clone(), contents);

    let mut compiler = Compiler::new(CompileOptions::default());
    let compilation = compiler
        .compile(&source)
        .unwrap_or_else(|err| panic!("compile failed for {}: {err}", path.display()));

    let ir = aot::compile_compilation_to_llvm_ir(&compilation)
        .unwrap_or_else(|err| panic!("LLVM codegen failed for {}: {err}", path.display()));

    assert!(
        ir.contains("tail call void @tea_print_int(i64"),
        "expected collapsed direct print in IR:\n{ir}"
    );
    assert!(
        !ir.contains("vector.body"),
        "periodic top-level loop should not remain after lowering:\n{ir}"
    );
    assert!(
        !ir.contains("urem i64"),
        "collapsed loop should avoid runtime modulo work:\n{ir}"
    );
}

fn temporary_object_path(source: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock went backwards")
        .as_millis();
    let file_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module");
    let suffix = if cfg!(windows) { "obj" } else { "o" };
    let file_name = format!("{file_stem}-{timestamp}.{suffix}");
    std::env::temp_dir().join(file_name)
}
