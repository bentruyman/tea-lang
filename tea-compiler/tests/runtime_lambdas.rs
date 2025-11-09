use std::path::PathBuf;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};

#[test]
fn lambda_captures_local_variable() -> anyhow::Result<()> {
    let source = r#"


def run() -> Int
  var base = 40
  var add = |value: Int| => base + value
  add(2)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(SourceId(0), PathBuf::from("lambda.tea"), source.to_string());
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}

#[test]
fn anonymous_function_definition_works() -> anyhow::Result<()> {
    let source = r#"


def make_adder(x: Int) -> Func(Int) -> Int
  return def(y: Int) -> Int
    return x + y
  end
end

def run() -> Int
  var adder = make_adder(42)
  adder(10)
end

print(run())
"#;

    let mut compiler = Compiler::new(CompileOptions::default());
    let source_file = SourceFile::new(
        SourceId(0),
        PathBuf::from("anon_func.tea"),
        source.to_string(),
    );
    compiler.compile(&source_file)?;
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // Full test execution support via AOT is planned for the future
    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics()
    );

    // TODO: Add AOT test execution when implemented
    // For now, we verify that the code compiles without errors

    Ok(())
}
