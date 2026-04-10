use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tea_compiler::{
    CompileOptions, CompileTarget, Compiler, InMemoryModuleLoader, SourceFile, SourceId,
};

fn browser_compiler(source_text: &str) -> (Compiler, SourceFile) {
    let entry_path = PathBuf::from("/main.tea");
    let loader = InMemoryModuleLoader::new(HashMap::from([(
        entry_path.clone(),
        source_text.to_string(),
    )]))
    .with_browser_stdlib();
    let compiler = Compiler::new(CompileOptions {
        target: CompileTarget::Browser,
        module_loader: Some(Arc::new(loader)),
        ..CompileOptions::default()
    });
    let source = SourceFile::new(SourceId(0), entry_path, source_text.to_string());
    (compiler, source)
}

#[test]
fn browser_target_expands_safe_source_stdlib() {
    let source = r#"
use string = "std.string"

@println(string.to_upper("tea"))
"#;

    let (mut compiler, source_file) = browser_compiler(source);
    compiler
        .compile(&source_file)
        .expect("browser compile to succeed");
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics().entries()
    );
}

#[test]
fn browser_target_rejects_native_only_modules() {
    let source = r#"
use fs = "std.fs"
"#;

    let (mut compiler, source_file) = browser_compiler(source);
    let error = compiler
        .compile(&source_file)
        .err()
        .expect("browser compile should fail");
    assert!(!error.to_string().is_empty(), "expected a non-empty error");
    assert!(
        compiler
            .diagnostics()
            .entries()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("std.fs")),
        "expected a std.fs diagnostic, found {:?}",
        compiler.diagnostics().entries()
    );
}
