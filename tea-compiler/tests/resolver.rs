use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

fn collect_diagnostics(compiler: &Compiler) -> Vec<String> {
    compiler
        .diagnostics()
        .entries()
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn rejects_use_of_undefined_binding() {
    let source = r#"

print(missing)
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("undefined.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected resolver to reject undefined binding"
    );

    let diagnostics = collect_diagnostics(&compiler);
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("use of undefined binding 'missing'")),
        "expected undefined binding diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn rejects_duplicate_declaration_in_same_scope() {
    let source = "var count = 1\nvar count = 2\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("duplicate.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected resolver to reject duplicate declaration"
    );

    let diagnostics = collect_diagnostics(&compiler);
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("duplicate declaration of variable 'count'")),
        "expected duplicate declaration diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn rejects_shadowing_outer_scope_binding() {
    let source = r#"
var total = 1
if total == 1
  var total = 2
end
"#;
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("shadow.tea"), source.to_string());
    let result = compiler.compile(&source_file);
    assert!(result.is_err(), "expected resolver to reject shadowing");

    let diagnostics = collect_diagnostics(&compiler);
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("shadows existing")),
        "expected shadowing diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn rejects_assignment_to_const() {
    let source = "const limit = 10\nlimit = 20\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("const_assign.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected resolver to reject const assignment"
    );

    let diagnostics = collect_diagnostics(&compiler);
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("cannot reassign const 'limit'")),
        "expected const reassignment diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn suggests_import_for_std_function() {
    let source = "to_string(42)\n";
    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("missing_import.tea"),
        source.to_string(),
    );
    let result = compiler.compile(&source_file);
    assert!(
        result.is_err(),
        "expected resolver to reject missing std import"
    );

    let diagnostics = compiler.diagnostics().entries();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("add `use intrinsics = \"std.intrinsics\"` to import it")
        }),
        "expected suggestion to import std.intrinsics, found {:?}",
        diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
}
